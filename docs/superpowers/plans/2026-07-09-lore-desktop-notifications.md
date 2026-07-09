# Real-time Notifications Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-07-09-lore-desktop-notifications-design.md`

**Goal:** Live refresh of status/locks/history driven by `lore notification subscribe`, with a discreet toast for teammates' pushes.

---

### Task 1: Rust — subscription sidecar

**Files:** Create `src-tauri/src/notifications.rs`; Modify `src-tauri/src/lib.rs`

- [x] **Step 1:** Create `src-tauri/src/notifications.rs`:

```rust
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
```

- [x] **Step 2:** `lib.rs` — `mod notifications;`, `.manage(notifications::NotifState::default())` before `.invoke_handler`, register `notifications::lore_notifications_start, notifications::lore_notifications_stop`.

- [x] **Step 3:** `cargo test` PASS → commit `feat(notifications): lore subscribe sidecar with auto-restart`.

---

### Task 2: Front — routing pur + API + watcher

**Files:** Create `src/lib/notifyRouting.ts`, `src/lib/notifyRouting.test.ts`, `src/lib/notifications.svelte.ts`; Modify `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/App.svelte`

- [x] **Step 1 (TDD):** `notifyRouting.test.ts` — push par un autre ⇒ status+history+toast ; push par moi ⇒ pas de toast ; lock ⇒ locks seulement ; mix coalescé. Puis :

```ts
/** Pure routing for coalesced server notifications. */
export interface LoreNotification { tagName: string; data: Record<string, unknown> }
export interface RefreshPlan {
  status: boolean
  locks: boolean
  /** Set when a teammate (not `myUserId`) pushed — worth a toast. */
  pushToast: { revisionNumber: number } | null
}

export function planFor(events: LoreNotification[], myUserId: string | null): RefreshPlan {
  const plan: RefreshPlan = { status: false, locks: false, pushToast: null }
  for (const e of events) {
    if (e.tagName === 'notificationBranchPushed') {
      plan.status = true
      const uid = typeof e.data.userId === 'string' ? e.data.userId : null
      if (uid && uid !== myUserId) plan.pushToast = { revisionNumber: Number(e.data.revisionNumber ?? 0) }
    } else if (e.tagName === 'notificationResourceLocked' || e.tagName === 'notificationResourceUnlocked') {
      plan.locks = true
    }
  }
  return plan
}
```

- [x] **Step 2:** types.ts — ré-exporter `LoreNotification` (import depuis notifyRouting) et ajouter à `LoreApi` :

```ts
  /** Live server events for the repo; resolves to a stop function. */
  startNotifications(repoPath: string, onEvent: (e: LoreNotification) => void): Promise<() => void>
```

- [x] **Step 3:** tauri.ts —

```ts
  startNotifications: async (repoPath, onEvent) => {
    const unlisten = await listen<LoreNotification>('lore://notification', (e) => onEvent(e.payload))
    await invoke('lore_notifications_start', { repoPath })
    return () => {
      unlisten()
      invoke('lore_notifications_stop').catch(() => { /* app closing */ })
    }
  },
```

(import `listen` from `@tauri-apps/api/event`.) — mock.ts : `async startNotifications() { return () => {} }`.

- [x] **Step 4:** `notifications.svelte.ts` :

```ts
import { api } from './api'
import { session } from './session.svelte'
import { refreshStatus, refreshLocks, refreshHistory } from './repo.svelte'
import { planFor, type LoreNotification } from './notifyRouting'
import { toasts } from './toast' // non — utiliser toastAction/toastError? -> toast info simple

let stop: (() => void) | null = null
let watchToken = 0
let pending: LoreNotification[] = []
let timer: ReturnType<typeof setTimeout> | null = null

/** Subscribe to the given repo's live events (null = unsubscribe). Idempotent. */
export async function watchRepo(repoPath: string | null) {
  const token = ++watchToken
  stop?.()
  stop = null
  if (!repoPath) return
  const s = await api.startNotifications(repoPath, onEvent)
  if (token !== watchToken) { s(); return } // superseded while awaiting
  stop = s
}

function onEvent(e: LoreNotification) {
  pending.push(e)
  if (timer) clearTimeout(timer)
  timer = setTimeout(flush, 400) // coalesce bursts into one refresh round
}

function flush() {
  timer = null
  const events = pending
  pending = []
  const plan = planFor(events, session.identity?.id ?? null)
  if (plan.status) { refreshStatus(true); refreshHistory(true) }
  if (plan.locks) refreshLocks(true)
  if (plan.pushToast) toastInfo(`Rev ${plan.pushToast.revisionNumber} pushed by a teammate`)
}
```

Toast : vérifier l'API de `toast.ts` (il y a `toastError`/`toastAction`) — ajouter si besoin un `toastInfo(title)` minimal (variant info sans action) dans toast.ts + test.

- [x] **Step 5:** App.svelte —

```ts
  $effect(() => {
    watchRepo(session.signedIn ? session.config.currentRepo : null)
  })
```

- [x] **Step 6:** `npm run check && npm test` PASS → commit `feat(notifications): live status/locks/history refresh with teammate push toast`.

---

### Task 3: Vérification simulation d'équipe

- [x] App réelle ouverte sur `lore-test-repo`, **sans y toucher** : `lore lock acquire README.md` depuis le CLI → la StatusBar/vue Locks se met à jour seule (≤ ~1 s) ; `lore lock release` → idem. Commit + push CLI d'un fichier → badge Sync apparaît seul.
- [x] Suites complètes PASS ; commit fixes éventuels.
