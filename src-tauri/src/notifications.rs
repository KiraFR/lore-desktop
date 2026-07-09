use std::io::BufRead;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tauri::{Emitter, Manager};

/// Serial number of the active subscription; a reader whose generation is
/// stale exits without respawning, so rapid stop/start never races.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The current subscriber child, tagged with its generation.
#[derive(Default)]
pub struct NotifState(pub Mutex<Option<(u64, std::process::Child)>>);

/// Forward real notifications; drop the handshake event.
fn is_forwardable(tag: &str) -> bool {
    tag.starts_with("notification") && tag != "notificationSubscribed"
}

fn spawn_subscriber(repo_path: &str) -> std::io::Result<std::process::Child> {
    let mut cmd = std::process::Command::new("lore");
    cmd.args(["notification", "subscribe", "--repository", repo_path, "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.spawn()
}

fn kill_slot(slot: &mut Option<(u64, std::process::Child)>) {
    if let Some((_, mut child)) = slot.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Subscribe to the repo's server events and forward them to the webview as
/// `lore://notification` events. Restarts the stream (3 s pause) if it drops.
#[tauri::command]
pub fn lore_notifications_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, NotifState>,
    repo_path: String,
) -> Result<(), String> {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    kill_slot(&mut state.0.lock().unwrap());

    std::thread::spawn(move || {
        while GENERATION.load(Ordering::SeqCst) == generation {
            let mut child = match spawn_subscriber(&repo_path) {
                Ok(c) => c,
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }
            };
            let Some(stdout) = child.stdout.take() else { break };
            {
                let st = app.state::<NotifState>();
                let mut slot = st.0.lock().unwrap();
                kill_slot(&mut slot);
                *slot = Some((generation, child));
            }
            for line in std::io::BufReader::new(stdout).lines() {
                if GENERATION.load(Ordering::SeqCst) != generation {
                    break;
                }
                let Ok(line) = line else { break };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
                let tag = v.get("tagName").and_then(|t| t.as_str()).unwrap_or("");
                if is_forwardable(tag) {
                    let _ = app.emit("lore://notification", &v);
                }
            }
            // Stream over (kill, server restart, network). Reap our own child
            // only — a newer generation may already own the slot.
            {
                let st = app.state::<NotifState>();
                let mut slot = st.0.lock().unwrap();
                if matches!(&*slot, Some((g, _)) if *g == generation) {
                    kill_slot(&mut slot);
                }
            }
            if GENERATION.load(Ordering::SeqCst) != generation {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });
    Ok(())
}

#[tauri::command]
pub fn lore_notifications_stop(state: tauri::State<'_, NotifState>) {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    kill_slot(&mut state.0.lock().unwrap());
}

#[cfg(test)]
mod tests {
    use super::is_forwardable;

    #[test]
    fn forwards_real_notifications_only() {
        assert!(is_forwardable("notificationBranchPushed"));
        assert!(is_forwardable("notificationResourceLocked"));
        assert!(!is_forwardable("notificationSubscribed"));
        assert!(!is_forwardable("complete"));
    }
}
