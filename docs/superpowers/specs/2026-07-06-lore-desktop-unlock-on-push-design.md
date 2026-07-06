# Lore Desktop — Unlock Pushed Files After Push

**Date:** 2026-07-06
**Status:** Approved for implementation

## Goal

After a successful push, if the user holds locks on files that were part of that push,
offer to release those locks — the lock-workflow's natural "I'm done, others can edit now"
step. Only the files in *this* push are offered (not every lock the user holds).

## Scope

In scope: a backend command that computes "files I hold locked that are in the pending
push", an action-capable toast, and wiring the push flow to prompt release after success.

Out of scope: auto-release (always opt-in via the toast button); releasing locks on files
not in the push; a full modal dialog (the action toast is the surface).

## CLI ground truth (captured on `lore-test-repo`)

`lore diff --source <revA> --target <revB> --json` (no paths) emits one `fileDiff` event
per changed file, each with a `path` field — the changeset between two revisions. Verified
`diff --source <rev3> --target <rev4>` → `fileDiff {path: "notes.txt"}`.

`lore status --json`'s `repositoryStatusRevision` carries `revisionRemote` and
`revisionLocal` (tip hashes). Before a push, `diff --source revisionRemote --target
revisionLocal` is exactly the set about to be pushed. (Both the diff and the revisions
require remote auth — see the SSO token-expiry note in the memory.)

## Architecture

### Rust — `src-tauri/src/commands.rs`

```rust
/// File paths changed in a revision-range diff (`fileDiff` events).
fn pushed_paths_from(events: &[LoreEvent]) -> std::collections::HashSet<String> { /* see plan */ }

/// Files the signed-in user holds locked AND that are part of the pending push
/// (the diff between the remote tip and the local tip). Empty when nothing is
/// ahead, on a first push with no remote tip yet all held locks qualify.
#[tauri::command]
pub fn lore_pushed_lock_files(repo_path: String) -> Result<Vec<String>, String> {
    // 1. my held locks (holder == "you"); short-circuit if none
    // 2. status → revisionRemote / revisionLocal; short-circuit if equal (nothing to push)
    // 3. first push (remote tip empty / all-zero) → every held lock is being pushed → return mine
    // 4. else diff(remote → local) → pushed set; return mine ∩ pushed
}
```

Reuses `locks_from` + `current_user_id` (own locks report holder `"you"`) and
`json`-field access already in the file. Registered in `lib.rs`.

### Frontend

`src/lib/toast.ts` — extend the toast model with a variant and an optional action:
```ts
export interface ToastAction { label: string; run: () => void }
export interface Toast { id; title; message; variant: 'error' | 'info'; action?: ToastAction }
export function toastAction(title: string, action: ToastAction): number // variant 'info', 15s TTL
```
`toastError` keeps its 6s TTL and gains `variant: 'error'`.

`src/lib/Toaster.svelte` — an `info` variant renders with the accent colour (not red) and,
when `action` is present, an accent button that runs `action.run()` then dismisses the toast.

`src/lib/types.ts` — `LoreApi.pushedLockFiles(repoPath): Promise<string[]>`.
`src/lib/tauri.ts` — `pushedLockFiles: (repoPath) => invoke('lore_pushed_lock_files', { repoPath })`.
`src/lib/mock.ts` — returns the mock locks held by "you" (stand-in for the diff intersection).

`src/lib/repo.svelte.ts`:
- `releaseLocks(paths: string[])` — `setLock(path, false)` for each (toast on per-file failure),
  then `refreshStatus()` + `refreshLocks()`.
- `push` computes `pushedLockFiles` **before** the push (revisions still divergent),
  pushes, and on success — if the set is non-empty — raises a `toastAction`
  *"N locked file(s) pushed"* / **Release** → `releaseLocks(set)`. The pre-push query is
  best-effort: a failure there never blocks the push.

## Data flow

Push button → (before push) `pushedLockFiles` = held locks ∩ pending diff → `api.push` →
on success, action toast → user clicks **Release** → locks released → status/locks refresh.

## Testing

- **Rust unit test** for `pushed_paths_from`: a captured `fileDiff` slice → the path set.
- **vitest**: `toastAction` adds an `info` toast carrying the action; `mock.pushedLockFiles`
  returns the "you"-held lock paths.
- **E2E (browser mock + `tauri dev`)**: hold a lock, commit, push → the action toast appears;
  **Release** clears the lock (verified in the Locks view / file preview).
