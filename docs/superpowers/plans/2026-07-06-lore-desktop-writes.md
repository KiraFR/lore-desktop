# Lore Desktop Slice D — Writes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (small change, inline). Steps use `- [ ]`.

**Goal:** Wire `commitAll` / `push` / `sync` / `setLock` to the real `lore` CLI via four thin Rust commands, so the app performs real writes instead of silently running the mock.

**Architecture:** Four `#[tauri::command]`s in `src-tauri/src/commands.rs` shell `lore … --repository <path>` via the existing `run_lore` helper; registered in `lib.rs`; `tauri.ts` overrides the four `LoreApi` methods. `mock.ts`/`repo.svelte.ts`/`Changes.svelte` unchanged.

**Tech Stack:** Tauri v2 (Rust), the `lore` CLI, Svelte/TS. Reuses `run_lore` (checks terminal `complete.status`); no new deps/capabilities.

**Branch:** `wiring-slice-d-writes`. Repo root: `C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop`.

**Safety:** arg arrays are literal — never pass `--reset` (destructive sync), `--force`, or `--fast-forward-merge`.

---

## Task 1: Rust write commands

**Files:** Modify `src-tauri/src/commands.rs` (append after `lore_sign_in`, before the test modules); Modify `src-tauri/src/lib.rs`.

- [ ] **Step 1: Write the failing tests** — append a test module to `commands.rs`:

```rust
#[cfg(test)]
mod writes_tests {
    use super::*;

    #[test]
    fn lock_subcommand_maps_bool() {
        assert_eq!(lock_subcommand(true), "acquire");
        assert_eq!(lock_subcommand(false), "release");
    }

    #[test]
    fn commit_rejects_empty_message() {
        // Whitespace-only message must be rejected before any lore call.
        let err = lore_commit("C:/nonexistent-repo".into(), "   ".into()).unwrap_err();
        assert!(err.contains("message"), "err was {err}");
    }
}
```

- [ ] **Step 2: Run — expect FAIL** (`lock_subcommand`/`lore_commit` not defined): `cargo test --manifest-path src-tauri/Cargo.toml writes_tests`

- [ ] **Step 3: Implement** — append to `commands.rs` immediately after `lore_sign_in`'s closing brace (before `#[cfg(test)]`):

```rust
/// `lore lock` subcommand for a lock/unlock toggle.
fn lock_subcommand(lock: bool) -> &'static str {
    if lock {
        "acquire"
    } else {
        "release"
    }
}

/// Stage the whole working tree then commit it. Selective staging is a follow-up;
/// this commits everything, matching the current UI (checkboxes are decorative).
#[tauri::command]
pub fn lore_commit(repo_path: String, message: String) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("commit message is required".to_string());
    }
    run_lore(&["stage", ".", "--scan", "--repository", &repo_path])?;
    run_lore(&["commit", &message, "--repository", &repo_path])?;
    Ok(())
}

#[tauri::command]
pub fn lore_push(repo_path: String) -> Result<(), String> {
    run_lore(&["push", "--repository", &repo_path])?;
    Ok(())
}

/// Plain `lore sync` — pulls/merges the remote into the local branch
/// non-destructively (NO `--reset`, which would discard local modifications).
#[tauri::command]
pub fn lore_sync(repo_path: String) -> Result<(), String> {
    run_lore(&["sync", "--repository", &repo_path])?;
    Ok(())
}

#[tauri::command]
pub fn lore_set_lock(repo_path: String, path: String, lock: bool) -> Result<(), String> {
    run_lore(&["lock", lock_subcommand(lock), &path, "--repository", &repo_path])?;
    Ok(())
}
```

- [ ] **Step 4: Register in `lib.rs`** — add to the `tauri::generate_handler![…]` list (after `config::config_save,`):

```rust
        commands::lore_commit,
        commands::lore_push,
        commands::lore_sync,
        commands::lore_set_lock,
```

- [ ] **Step 5: Run — expect PASS** + build: `cargo test --manifest-path src-tauri/Cargo.toml writes_tests` then `cargo build --manifest-path src-tauri/Cargo.toml`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(wiring): lore_commit/push/sync/set_lock commands"
```

## Task 2: Wire `tauri.ts`

**Files:** Modify `src/lib/tauri.ts`.

- [ ] **Step 1: Add the four overrides** — inside the `tauriApi` object (after `saveConfig`):

```ts
  commitAll: (repoPath, message) => invoke<void>('lore_commit', { repoPath, message }),
  push: (repoPath) => invoke<void>('lore_push', { repoPath }),
  sync: (repoPath) => invoke<void>('lore_sync', { repoPath }),
  setLock: (repoPath, path, lock) => invoke<void>('lore_set_lock', { repoPath, path, lock }),
```

- [ ] **Step 2: Typecheck + tests** — `npm run check` (0/0) and `npm test` (10/10, unchanged).

- [ ] **Step 3: Commit**

```bash
git add src/lib/tauri.ts
git commit -m "feat(wiring): commitAll/push/sync/setLock over invoke (real writes)"
```

## Task 3: Merge + E2E

- [ ] Merge `wiring-slice-d-writes` → `main` + push.
- [ ] **E2E (user-assisted, `tauri dev`)** on `C:/Users/jimmy/lore-test-repo`: edit a tracked file → commit (status clean, ahead+1) → push (ahead 0) → sync (ok) → lock a file (chip "you") → unlock.
