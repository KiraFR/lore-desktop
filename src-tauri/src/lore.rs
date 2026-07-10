use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct LoreEvent {
    #[serde(rename = "tagName")]
    pub tag_name: String,
    pub data: Value,
}

/// Parse an NDJSON stream (one JSON object per line). Non-JSON lines are
/// skipped. Returns every event in order. Errors only on a malformed stream
/// where no `complete` event is present.
pub fn parse_events(stdout: &str) -> Result<Vec<LoreEvent>, String> {
    let mut events = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(ev) = serde_json::from_str::<LoreEvent>(line) {
            events.push(ev);
        }
    }
    if !events.iter().any(|e| e.tag_name == "complete") {
        return Err("lore did not emit a completion event".to_string());
    }
    Ok(events)
}

/// Return Err(message) if the terminal `complete` status is non-zero or an
/// `error` event was emitted.
pub fn check_ok(events: &[LoreEvent]) -> Result<(), String> {
    if let Some(err) = events.iter().find(|e| e.tag_name == "error") {
        return Err(err.data.to_string());
    }
    let status = events
        .iter()
        .rev()
        .find(|e| e.tag_name == "complete")
        .and_then(|e| e.data.get("status"))
        .and_then(|s| s.as_i64())
        .unwrap_or(-1);
    if status != 0 {
        return Err(format!("lore exited with status {status}"));
    }
    Ok(())
}

pub fn events_with_tag<'a>(events: &'a [LoreEvent], tag: &str) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|e| e.tag_name == tag)
        .map(|e| &e.data)
        .collect()
}

use std::process::Command;
use std::time::Duration;

/// Hard cap on any single `lore` invocation. A remote call that hangs (offline /
/// transport error) must never wedge a command indefinitely — it errors instead.
const LORE_TIMEOUT: Duration = Duration::from_secs(45);

/// Run `lore <args> --json`, capturing stdout. `--json` is appended here so
/// callers pass only the subcommand + options. The process runs on a helper
/// thread with a timeout so a hung remote call returns an error rather than
/// blocking forever. (Commands are async + `spawn_blocking`, so this never runs
/// on the UI thread.)
pub fn run_lore(args: &[&str]) -> Result<Vec<LoreEvent>, String> {
    let mut owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    owned.push("--json".to_string());
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(Command::new("lore").args(&owned).output());
    });
    let output = match rx.recv_timeout(LORE_TIMEOUT) {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err(format!("failed to launch lore: {e}")),
        Err(_) => return Err("lore command timed out".to_string()),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let events = parse_events(&stdout)?;
    check_ok(&events)?;
    Ok(events)
}

/// Stall detector for the streaming runner: if the child emits NO line for this
/// long, it is considered hung, killed, and the operation errors. An operation
/// that keeps making progress is never killed — this replaces the flat 45 s cap
/// for clone/sync/push, which legitimately run for minutes on studio binaries.
///
/// Assumes lore emits chunk-granular progress during transfers. UNVERIFIED
/// against a real clone (capture blocked — server unreachable): if progress
/// turns out to be per-file, a single multi-GB asset transfer could
/// stall-kill legitimately — revisit this constant (or make it per-operation)
/// with the Task 13 capture.
pub const LORE_STALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Run `lore <args> --json` streaming stdout line by line. Each NDJSON event is
/// (a) forwarded to `on_event` as it arrives and (b) collected for the final
/// result, validated by the same complete-event + `check_ok` rules as
/// `run_lore`. Modeled on the notifications sidecar (notifications.rs).
#[allow(dead_code)] // wired in by the op-progress relay (Task 15)
pub fn run_lore_streaming(
    args: &[&str],
    on_event: &mut dyn FnMut(&LoreEvent),
) -> Result<Vec<LoreEvent>, String> {
    let mut owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    owned.push("--json".to_string());
    let owned_refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    run_streaming_cmd("lore", &owned_refs, LORE_STALL_TIMEOUT, on_event)
}

/// Program-agnostic core of the streaming runner (testable with a fake child).
fn run_streaming_cmd(
    program: &str,
    args: &[&str],
    stall: Duration,
    on_event: &mut dyn FnMut(&LoreEvent),
) -> Result<Vec<LoreEvent>, String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn().map_err(|e| format!("failed to launch {program}: {e}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "no stdout pipe".to_string())?;
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        use std::io::BufRead;
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break; // receiver gone (stall kill) — stop reading
            }
        }
    });
    let result = collect_streaming(&rx, stall, on_event);
    if result.is_err() {
        let _ = child.kill(); // stall or bad stream — don't leave a zombie
    }
    let status = child.wait().ok();
    result.map_err(|e| match status {
        // Not for the stall path (there the exit code is just our own kill).
        Some(s) if !s.success() && !e.contains("no progress") => format!("{e} (lore exited with {s})"),
        _ => e,
    })
}

/// Drain the line channel with a stall timeout, parsing + relaying each event.
/// Channel disconnect = clean EOF; a silent-but-alive sender = a stalled child.
fn collect_streaming(
    rx: &std::sync::mpsc::Receiver<String>,
    stall: Duration,
    on_event: &mut dyn FnMut(&LoreEvent),
) -> Result<Vec<LoreEvent>, String> {
    let mut events: Vec<LoreEvent> = Vec::new();
    let mut skipped: usize = 0;
    loop {
        match rx.recv_timeout(stall) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(ev) = serde_json::from_str::<LoreEvent>(line) {
                    on_event(&ev);
                    // Progress events from a very large clone are all retained
                    // here (transiently up to tens of MB in the worst case);
                    // callers (check_ok/events_with_tag) only need the
                    // non-progress events. Worth trimming if a profile ever
                    // shows this mattering — deferred until then.
                    events.push(ev);
                } else {
                    skipped += 1;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if events.iter().any(|e| e.tag_name == "complete") {
                    // The child already reported completion; the pipe just
                    // never reached EOF (a helper process inherited the
                    // stdout handle and is still holding it open). That is
                    // not a stall — the operation itself succeeded.
                    break;
                }
                return Err(format!(
                    "lore made no progress for {} s — operation aborted",
                    stall.as_secs().max(1)
                ));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if !events.iter().any(|e| e.tag_name == "complete") {
        return Err(if skipped > 0 {
            format!("lore did not emit a completion event ({skipped} unparseable lines skipped)")
        } else {
            "lore did not emit a completion event".to_string()
        });
    }
    check_ok(&events)?;
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_checks_ok() {
        let sample = concat!(
            r#"{"tagName":"authIdentity","data":{"userId":"u1"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        assert_eq!(events_with_tag(&events, "authIdentity").len(), 1);
        assert!(check_ok(&events).is_ok());
    }

    #[test]
    fn non_zero_status_is_error() {
        let sample = concat!(
            r#"{"tagName":"error","data":{"errorInner":"nope"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":1}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        assert!(check_ok(&events).is_err());
    }

    #[test]
    fn streaming_relays_events_incrementally_in_order() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        tx.send(r#"{"tagName":"repositoryCloneProgress","data":{"done":10,"total":100}}"#.into()).unwrap();
        tx.send(r#"{"tagName":"repositoryCloneProgress","data":{"done":100,"total":100}}"#.into()).unwrap();
        tx.send(r#"{"tagName":"complete","data":{"status":0}}"#.into()).unwrap();
        drop(tx);
        let mut seen: Vec<String> = Vec::new();
        let events = collect_streaming(&rx, Duration::from_millis(500), &mut |ev| seen.push(ev.tag_name.clone())).unwrap();
        assert_eq!(seen, ["repositoryCloneProgress", "repositoryCloneProgress", "complete"]);
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn streaming_requires_a_complete_event() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        tx.send(r#"{"tagName":"repositoryCloneProgress","data":{"done":10,"total":100}}"#.into()).unwrap();
        drop(tx); // stream ends without complete
        let err = collect_streaming(&rx, Duration::from_millis(500), &mut |_| {}).unwrap_err();
        assert!(err.contains("completion"), "err was {err}");
    }

    #[test]
    fn no_completion_error_reports_skipped_garbage_lines() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        tx.send("not json at all".into()).unwrap();
        tx.send("still not json".into()).unwrap();
        drop(tx); // stream ends without complete
        let err = collect_streaming(&rx, Duration::from_millis(500), &mut |_| {}).unwrap_err();
        assert!(err.contains("2 unparseable"), "err was {err}");
    }

    #[test]
    fn streaming_silence_is_a_stall_error() {
        let (_tx, rx) = std::sync::mpsc::channel::<String>();
        // Sender alive but silent (fake hung child) → stall, not disconnect.
        let err = collect_streaming(&rx, Duration::from_millis(50), &mut |_| {}).unwrap_err();
        assert!(err.contains("no progress"), "err was {err}");
    }

    #[test]
    fn timeout_after_complete_is_success_not_stall() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        tx.send(r#"{"tagName":"repositoryCloneProgress","data":{"done":10,"total":100}}"#.into()).unwrap();
        tx.send(r#"{"tagName":"complete","data":{"status":0}}"#.into()).unwrap();
        // NOTE: `tx` is deliberately kept alive (not dropped) — this reproduces
        // a child that emitted `complete` but whose pipe never reaches EOF
        // because a helper process inherited the stdout handle.
        let events = collect_streaming(&rx, Duration::from_millis(50), &mut |_| {}).unwrap();
        assert_eq!(events.len(), 2);
        drop(tx);
    }

    #[test]
    fn streaming_error_event_fails_check() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        tx.send(r#"{"tagName":"error","data":{"errorInner":"nope"}}"#.into()).unwrap();
        tx.send(r#"{"tagName":"complete","data":{"status":1}}"#.into()).unwrap();
        drop(tx);
        assert!(collect_streaming(&rx, Duration::from_millis(500), &mut |_| {}).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn dead_child_error_includes_exit_status() {
        // No stdout at all → the reader thread hits EOF immediately
        // (Disconnected) without ever seeing a `complete` event.
        let err = run_streaming_cmd(
            "powershell",
            &["-NoProfile", "-Command", "exit 3"],
            Duration::from_millis(500),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(err.contains("completion"), "err was {err}");
        assert!(err.contains("exited"), "err was {err}");
    }

    #[test]
    #[cfg(windows)]
    fn stalled_child_is_killed_promptly() {
        // A real child that prints nothing: the stall detector must kill it and
        // return well before its natural 30 s lifetime.
        let start = std::time::Instant::now();
        let err = run_streaming_cmd(
            "powershell",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            Duration::from_millis(300),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(err.contains("no progress"), "err was {err}");
        assert!(start.elapsed() < Duration::from_secs(10), "child was not killed promptly");
    }
}
