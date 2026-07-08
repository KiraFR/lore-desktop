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
}
