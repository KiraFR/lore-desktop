# Lore Desktop — Slice D: writes (commit / push / sync / lock)

- **Date:** 2026-07-06
- **Repo:** github.com/KiraFR/lore-desktop, branch `wiring-slice-d-writes`
- **Ticket:** TICKET-127 (Lore Desktop)
- **Status:** design approved; next = writing-plans → inline implementation
- **Builds on:** Slice A/B/C (mock→real Tauri-command pipeline; `run_lore` NDJSON helper; `api.ts` picks `tauriApi` inside Tauri else `mock`).

## Problem

`src/lib/tauri.ts` does not override `commitAll` / `push` / `sync` / `setLock`, so in the real Tauri app they fall through to the `...mock` spread and run the in-memory mock. The Changes/TitleBar buttons **appear** to work but never touch the real repo or server — a silent no-op.

## Goal

Wire commit / push / sync / lock+unlock to the real `lore` CLI via typed Rust commands, so the app performs real writes. **Commit stages and commits the whole working tree** (selective staging and merge are separate future slices).

## Architecture (Slice A pattern, unchanged)

Four new Rust `#[tauri::command]`s in `src-tauri/src/commands.rs` shell `lore … --json --repository <path>` via the existing `run_lore` helper (which returns `Err` on a non-zero terminal `complete.status`). `tauri.ts` overrides the four `LoreApi` methods with `invoke`. `mock.ts`, `repo.svelte.ts`, and `Changes.svelte` are unchanged — the existing wrapper flow (`act()` busy-state + `refreshStatus()` after success + `toastError` on failure; `setLock` also `refreshLocks()`) already fits. `api.ts`'s mock↔tauri swap is unchanged.

## Command mapping

| `LoreApi` method | `lore` invocation |
|---|---|
| `commitAll(repoPath, message)` | reject an empty message, then `lore stage . --scan --repository <repoPath>` **then** `lore commit <message> --repository <repoPath>` |
| `push(repoPath)` | `lore push --repository <repoPath>` (current branch) |
| `sync(repoPath)` | `lore sync --repository <repoPath>` (plain — non-destructive) |
| `setLock(repoPath, path, lock)` | `lore lock acquire <path> --repository <repoPath>` when `lock`, else `lore lock release <path> --repository <repoPath>` |

## Safety

The Rust commands build the argument arrays literally, so aggressive/destructive flags are never passed:
- **No `--reset` on sync** — `--reset` discards local modified files (confirmed `lore-server` source); plain `lore sync` is the non-destructive pull/merge-remote-into-local.
- **No `--force`**, **no `--fast-forward-merge`** on push.

A non-zero `complete.status` (empty message rejected by the CLI, diverged sync with conflicts, push rejected, a lock already held by someone else) makes `run_lore` return `Err`, surfaced through the existing toast path. Conflict **resolution** for a diverged sync is out of scope (the merge slice).

## Rust design (`commands.rs`)

- Pure helper `lock_subcommand(lock: bool) -> &'static str` → `"acquire"` / `"release"` (unit-tested).
- `lore_commit(repo_path: String, message: String) -> Result<(), String>`: if `message.trim().is_empty()` return `Err("commit message is required".into())` (early, no shell); else `run_lore(&["stage", ".", "--scan", "--repository", &repo_path])?;` then `run_lore(&["commit", &message, "--repository", &repo_path])?;` `Ok(())`.
- `lore_push(repo_path: String) -> Result<(), String>`: `run_lore(&["push", "--repository", &repo_path])?; Ok(())`.
- `lore_sync(repo_path: String) -> Result<(), String>`: `run_lore(&["sync", "--repository", &repo_path])?; Ok(())`.
- `lore_set_lock(repo_path: String, path: String, lock: bool) -> Result<(), String>`: `run_lore(&["lock", lock_subcommand(lock), &path, "--repository", &repo_path])?; Ok(())`.
- All four registered in `lib.rs`'s `generate_handler!`.

## Frontend (`tauri.ts`)

Add four overrides:
```ts
commitAll: (repoPath, message) => invoke<void>('lore_commit', { repoPath, message }),
push: (repoPath) => invoke<void>('lore_push', { repoPath }),
sync: (repoPath) => invoke<void>('lore_sync', { repoPath }),
setLock: (repoPath, path, lock) => invoke<void>('lore_set_lock', { repoPath, path, lock }),
```
`mock.ts` (browser dev), `repo.svelte.ts`, and `Changes.svelte` are unchanged.

## Testing

- **Rust unit tests:** `lock_subcommand(true) == "acquire"` and `(false) == "release"`; `lore_commit(<any path>, "   ")` returns `Err("commit message is required")` without shelling `lore` (the guard returns before the first `run_lore`). The stage/commit/push/sync/lock success paths shell `lore` and are covered by E2E, not unit tests.
- **E2E in `tauri dev`** on `C:/Users/jimmy/lore-test-repo`: edit a tracked file → **commit** with a message → status shows clean + `localAhead` incremented → **push** → ahead 0 → **sync** succeeds → **lock** a file → its lock chip shows "you" → **unlock**. (This performs real writes against the test repo + server, which is intended.)

## Out of scope

Merge (branch merge start/into/resolve/abort + a diff-based conflict preview); selective staging (wiring `Changes.svelte`'s checkboxes to a `stage(paths)` API — commit stays whole-tree here); `getLocks`/`getBranches` real wiring (Slice E); using the Changes description textarea as part of the commit message.
