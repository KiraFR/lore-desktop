# Lore Desktop — Wiring Slice B Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the repository picker real — list a server's repositories, open an already-cloned local working copy through a native folder dialog, and clone a selected server repository — and route every failed backend call to a red error toast.

**Architecture:** Two new typed Rust `#[tauri::command]`s (`lore_repositories`, `lore_clone`) shell `lore … --json` and reuse the Slice A `lore` NDJSON module. A native folder dialog is added via `@tauri-apps/plugin-dialog`, reached from `tauri.ts` and exposed as `LoreApi.pickFolder()`. A small `writable`-backed toast store (`toast.ts`) plus a `Toaster.svelte` overlay surface errors app-wide. `api.ts` keeps swapping `mock` ↔ `tauriApi`; components stay backend-agnostic.

**Tech Stack:** Tauri v2 (Rust), Svelte 5 + TypeScript, Vitest, the `lore` CLI (v0.8.3+201).

**Branch:** `wiring-slice-b` (already checked out; carries the spec).

**Conventions:** All UI text is English. No `Co-Authored-By: Claude` trailer on commits. All commands below run from the repo root `C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop` unless stated otherwise.

---

## File Structure

**Backend (`src-tauri/`):**
- `tests/fixtures/repo_list.ndjson` — **new**; captured real `repository list --json` output (test oracle).
- `src/commands.rs` — **modify**; add `RepoEntryDto` + `repositories_from` + `lore_repositories`; add `build_clone_args` + `lore_clone`; unit tests.
- `src/lib.rs` — **modify**; register the dialog plugin; add the two commands to the handler.
- `Cargo.toml` — **modify**; add `tauri-plugin-dialog`.
- `capabilities/default.json` — **modify**; grant `dialog:allow-open`.

**Frontend (`src/`):**
- `lib/toast.ts` — **new**; `writable`-backed toast store + `toastError` / `dismissToast`.
- `lib/toast.test.ts` — **new**; store unit tests.
- `lib/Toaster.svelte` — **new**; red toast stack, bottom-right.
- `App.svelte` — **modify**; mount `<Toaster />`.
- `lib/types.ts` — **modify**; add `pickFolder` + `cloneRepo` to `LoreApi`.
- `lib/mock.ts` — **modify**; add mock `pickFolder` + `cloneRepo`.
- `lib/tauri.ts` — **modify**; real `listRepos` / `pickFolder` / `cloneRepo`.
- `lib/mock.test.ts` — **modify**; cover the two new mock methods.
- `lib/RepoPicker.svelte` — **modify**; real open-folder + clone with toast errors.
- `lib/repo.svelte.ts` — **modify**; route status/commit/push/sync/lock errors to `toastError`.
- `lib/session.svelte.ts` — **modify**; guard `bootstrap` with `toastError`.
- `lib/SignIn.svelte` — **modify**; route the sign-in failure to `toastError`.
- `lib/Changes.svelte` — **modify**; drop the now-dead inline `repo.error` line.
- `package.json` — **modify**; add `@tauri-apps/plugin-dialog`.

---

## Task 1: Backend — `lore_repositories` (list server repositories)

**Files:**
- Create: `src-tauri/tests/fixtures/repo_list.ndjson`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the captured fixture**

Create `src-tauri/tests/fixtures/repo_list.ndjson` with this exact content (real output from `lore repository list lore://lore.example.com:41337 --json`):

```
{"tagName":"repositoryListEntry","data":{"id":"019f333af5e073d28bb117ad1596784a","name":"desktoptest1"}}
{"tagName":"repositoryListEntry","data":{"id":"019f2e1577257382bc89c5a28e3306cb","name":"ssotest11"}}
{"tagName":"repositoryListEntry","data":{"id":"019f2e14006f7870a7b27df367c78b72","name":"ssotest10"}}
{"tagName":"complete","data":{"status":0}}
```

- [ ] **Step 2: Write the failing test**

Append to `src-tauri/src/commands.rs`:

```rust
#[cfg(test)]
mod repositories_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_repo_list_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/repo_list.ndjson")).unwrap();
        let repos = repositories_from(&events);
        assert_eq!(repos.len(), 3);
        assert_eq!(repos[0].id, "019f333af5e073d28bb117ad1596784a");
        assert_eq!(repos[0].name, "desktoptest1");
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml repositories_tests`
Expected: FAIL — `cannot find function repositories_from` / `cannot find type RepoEntryDto`.

- [ ] **Step 4: Implement the DTO, mapper, and command**

Add to `src-tauri/src/commands.rs` (place next to the other DTOs, e.g. just before `CommitDto`):

```rust
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RepoEntryDto {
    pub id: String,
    pub name: String,
}

/// Map `repositoryListEntry` events (`{ id, name }`) onto the UI's `RepoEntry`.
fn repositories_from(events: &[LoreEvent]) -> Vec<RepoEntryDto> {
    events_with_tag(events, "repositoryListEntry")
        .into_iter()
        .map(|d| RepoEntryDto {
            id: d.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: d.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn lore_repositories(server_url: String) -> Result<Vec<RepoEntryDto>, String> {
    let events = run_lore(&["repository", "list", &server_url])?;
    Ok(repositories_from(&events))
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml repositories_tests`
Expected: PASS (`test repositories_tests::parses_repo_list_fixture ... ok`).

- [ ] **Step 6: Register the command**

In `src-tauri/src/lib.rs`, add `commands::lore_repositories,` to the `tauri::generate_handler![…]` list (after `commands::lore_history,`).

- [ ] **Step 7: Verify the whole crate still builds**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: build succeeds (warnings OK).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/tests/fixtures/repo_list.ndjson src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(wiring): lore_repositories command (real server repo list)"
```

---

## Task 2: Backend — `lore_clone` (clone a server repository)

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/src/commands.rs`:

```rust
#[cfg(test)]
mod clone_tests {
    use super::*;

    #[test]
    fn builds_clone_url_and_path() {
        let (url, path) = build_clone_args("lore://host:41337", "019abc", "desktoptest1", "C:/repos");
        assert_eq!(url, "lore://host:41337/019abc");
        // Path join uses the platform separator; assert both parts are present.
        assert!(path.ends_with("desktoptest1"), "path was {path}");
        assert!(path.contains("repos"), "path was {path}");
    }

    #[test]
    fn trims_trailing_slash_on_server_url() {
        let (url, _) = build_clone_args("lore://host:41337/", "id1", "n", "/tmp");
        assert_eq!(url, "lore://host:41337/id1");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml clone_tests`
Expected: FAIL — `cannot find function build_clone_args`.

- [ ] **Step 3: Implement the helper and command**

Add to `src-tauri/src/commands.rs`:

```rust
/// Build the `(clone URL, destination path)` pair for a clone.
/// The URL addresses the repository by id: `<server_url>/<repo_id>`.
/// The path is `<dest_parent>/<repo_name>`, joined with the platform separator.
fn build_clone_args(server_url: &str, repo_id: &str, repo_name: &str, dest_parent: &str) -> (String, String) {
    let url = format!("{}/{}", server_url.trim_end_matches('/'), repo_id);
    let path = std::path::Path::new(dest_parent)
        .join(repo_name)
        .to_string_lossy()
        .into_owned();
    (url, path)
}

/// Clone `<server_url>/<repo_id>` into `<dest_parent>/<repo_name>` and return the
/// created path. `run_lore` blocks until the clone finishes and errors on a
/// non-zero terminal `complete.status` — the picker shows that as a toast.
#[tauri::command]
pub fn lore_clone(
    server_url: String,
    repo_id: String,
    repo_name: String,
    dest_parent: String,
) -> Result<String, String> {
    let (url, path) = build_clone_args(&server_url, &repo_id, &repo_name, &dest_parent);
    run_lore(&["clone", &url, &path])?;
    Ok(path)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml clone_tests`
Expected: PASS (both tests `... ok`).

- [ ] **Step 5: Register the command**

In `src-tauri/src/lib.rs`, add `commands::lore_clone,` to the `tauri::generate_handler![…]` list (after `commands::lore_repositories,`).

- [ ] **Step 6: Verify the crate builds**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: build succeeds.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(wiring): lore_clone command (clone server repo to chosen path)"
```

---

## Task 3: Infra — native folder dialog plugin

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `package.json` (via `npm add`)

- [ ] **Step 1: Add the Rust plugin dependency**

In `src-tauri/Cargo.toml`, under `[dependencies]`, add:

```toml
tauri-plugin-dialog = "2"
```

- [ ] **Step 2: Register the plugin**

In `src-tauri/src/lib.rs`, add the plugin registration immediately after `tauri::Builder::default()`:

```rust
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| {
```

- [ ] **Step 3: Grant the dialog-open permission**

Replace the `permissions` array in `src-tauri/capabilities/default.json` so it reads:

```json
  "permissions": [
    "core:default",
    "dialog:allow-open"
  ]
```

- [ ] **Step 4: Add the JS plugin package**

Run: `npm add @tauri-apps/plugin-dialog`
Expected: `@tauri-apps/plugin-dialog` appears under `dependencies` in `package.json`; `package-lock.json` updates.

- [ ] **Step 5: Verify the Rust crate builds with the plugin**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: build succeeds (the plugin crate is fetched and compiled). Full dialog behavior is verified in the E2E task — it needs a real window.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/capabilities/default.json package.json package-lock.json
git commit -m "chore(wiring): add tauri-plugin-dialog + dialog:allow-open capability"
```

---

## Task 4: Frontend — app-wide toast system

**Files:**
- Create: `src/lib/toast.ts`
- Create: `src/lib/toast.test.ts`
- Create: `src/lib/Toaster.svelte`
- Modify: `src/App.svelte`

- [ ] **Step 1: Write the failing test**

Create `src/lib/toast.test.ts`:

```ts
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import { toasts, toastError, dismissToast } from './toast'

describe('toast store', () => {
  beforeEach(() => { toasts.set([]) })

  it('toastError adds a red toast with title + message', () => {
    toastError('Clone failed', new Error('boom'))
    const list = get(toasts)
    expect(list).toHaveLength(1)
    expect(list[0].title).toBe('Clone failed')
    expect(list[0].message).toBe('boom')
  })

  it('dismissToast removes the toast', () => {
    const id = toastError('Nope')
    dismissToast(id)
    expect(get(toasts)).toHaveLength(0)
  })

  it('auto-expires after the TTL', () => {
    vi.useFakeTimers()
    toastError('Nope')
    expect(get(toasts)).toHaveLength(1)
    vi.advanceTimersByTime(6000)
    expect(get(toasts)).toHaveLength(0)
    vi.useRealTimers()
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- toast`
Expected: FAIL — cannot resolve `./toast`.

- [ ] **Step 3: Implement the store**

Create `src/lib/toast.ts`:

```ts
import { writable } from 'svelte/store'

export interface Toast {
  id: number
  title: string
  /** Optional detail (the underlying error text); '' when absent. */
  message: string
}

/** Active toasts, newest last. A plain writable store so it is reactive in
 *  components and unit-testable without the Svelte compiler. */
export const toasts = writable<Toast[]>([])

let nextId = 1
const TOAST_TTL = 6000

/** Push a red error toast: a short title plus optional detail from `err`.
 *  Returns the toast id so callers/tests can dismiss it early. */
export function toastError(title: string, err?: unknown): number {
  const message = err === undefined ? '' : err instanceof Error ? err.message : String(err)
  const id = nextId++
  toasts.update((list) => [...list, { id, title, message }])
  setTimeout(() => dismissToast(id), TOAST_TTL)
  return id
}

export function dismissToast(id: number): void {
  toasts.update((list) => list.filter((t) => t.id !== id))
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npm test -- toast`
Expected: PASS (3 tests).

- [ ] **Step 5: Create the Toaster component**

Create `src/lib/Toaster.svelte`:

```svelte
<script lang="ts">
  import { toasts, dismissToast } from './toast'
  import Icon from './Icon.svelte'
</script>

<div class="toaster">
  {#each $toasts as t (t.id)}
    <div class="toast" role="alert">
      <span class="ico"><Icon name="alert" size={16} /></span>
      <div class="body">
        <strong class="title">{t.title}</strong>
        {#if t.message}<p class="msg">{t.message}</p>{/if}
      </div>
      <button class="close" onclick={() => dismissToast(t.id)} aria-label="Dismiss">×</button>
    </div>
  {/each}
</div>

<style>
  .toaster { position: fixed; right: 16px; bottom: 16px; z-index: 100; display: flex; flex-direction: column; gap: 8px; max-width: 380px; }
  .toast { display: flex; gap: 10px; align-items: flex-start; padding: 11px 12px; border-radius: var(--radius); background: var(--panel); border: 1px solid var(--deleted); border-left: 3px solid var(--deleted); box-shadow: 0 6px 20px rgba(0, 0, 0, .35); }
  .ico { color: var(--deleted); margin-top: 1px; }
  .body { min-width: 0; flex: 1; }
  .title { display: block; font-size: 13px; font-weight: 600; color: var(--text); }
  .msg { margin: 2px 0 0; font-size: 12px; color: var(--text-muted); word-break: break-word; }
  .close { background: none; border: none; color: var(--text-muted); font-size: 16px; line-height: 1; padding: 0 2px; }
  .close:hover { background: none; color: var(--text); }
</style>
```

- [ ] **Step 6: Mount the Toaster at the app root**

In `src/App.svelte`, add the import alongside the others:

```ts
  import Toaster from './lib/Toaster.svelte'
```

and render it as the last child of `<main class="shell">`, immediately before the closing `</main>`:

```svelte
    <StatusBar />
  {/if}
  <Toaster />
</main>
```

- [ ] **Step 7: Typecheck**

Run: `npm run check`
Expected: 0 errors, 0 warnings.

- [ ] **Step 8: Commit**

```bash
git add src/lib/toast.ts src/lib/toast.test.ts src/lib/Toaster.svelte src/App.svelte
git commit -m "feat(wiring): app-wide error toast system"
```

---

## Task 5: Frontend — `LoreApi` additions (`pickFolder`, `cloneRepo`, real `listRepos`)

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/mock.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/lib/mock.test.ts`

- [ ] **Step 1: Write the failing test**

In `src/lib/mock.test.ts`, add inside the `describe('mock api', …)` block:

```ts
  it('pickFolder returns a path; cloneRepo returns dest/name', async () => {
    const picked = await mock.pickFolder()
    expect(typeof picked).toBe('string')
    const cloned = await mock.cloneRepo('lore://demo:41337', 'id1', 'game-main', 'C:/repos')
    expect(cloned).toBe('C:/repos/game-main')
  })
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- mock`
Expected: FAIL — `mock.pickFolder is not a function` (and a TS error that `pickFolder`/`cloneRepo` are missing).

- [ ] **Step 3: Extend the `LoreApi` interface**

In `src/lib/types.ts`, add these two methods to the `LoreApi` interface, immediately after the `listRepos(serverUrl: string): Promise<RepoEntry[]>` line:

```ts
  /** Native OS directory chooser; returns the absolute path or null if cancelled. */
  pickFolder(): Promise<string | null>
  /** Clone <serverUrl>/<repoId> into <destParent>/<repoName>; returns the created path. */
  cloneRepo(serverUrl: string, repoId: string, repoName: string, destParent: string): Promise<string>
```

- [ ] **Step 4: Implement the mock methods**

In `src/lib/mock.ts`, add these two methods to the `mock` object, immediately after the `listRepos` method:

```ts
  async pickFolder() {
    await delay(120)
    return 'C:/SoonerOrLater/picked-repo'
  },
  async cloneRepo(_serverUrl: string, _repoId: string, repoName: string, destParent: string) {
    await delay(600) // simulate the network + disk work
    return `${destParent}/${repoName}`
  },
```

- [ ] **Step 5: Implement the Tauri methods**

Replace the entire contents of `src/lib/tauri.ts` with:

```ts
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { mock } from './mock'
import type { HistoryPage, LoreApi, RepoEntry, StatusResult } from './types'

export const tauriApi: LoreApi = {
  ...mock,
  isAuthenticated: () => invoke<boolean>('lore_is_authenticated'),
  signIn: (serverUrl, authUrlOverride) =>
    invoke<void>('lore_sign_in', { serverUrl, authUrl: authUrlOverride ?? null }),
  listRepos: (serverUrl) => invoke<RepoEntry[]>('lore_repositories', { serverUrl }),
  getStatus: (repoPath) => invoke<StatusResult>('lore_status', { repoPath }),
  getHistory: (repoPath, length, cursor) =>
    invoke<HistoryPage>('lore_history', { repoPath, length, cursor: cursor ?? null }),
  pickFolder: async () => {
    const picked = await open({ directory: true, multiple: false })
    return typeof picked === 'string' ? picked : null
  },
  cloneRepo: (serverUrl, repoId, repoName, destParent) =>
    invoke<string>('lore_clone', { serverUrl, repoId, repoName, destParent }),
}
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `npm test -- mock`
Expected: PASS (the new test plus the existing mock tests).

- [ ] **Step 7: Typecheck**

Run: `npm run check`
Expected: 0 errors, 0 warnings.

- [ ] **Step 8: Commit**

```bash
git add src/lib/types.ts src/lib/mock.ts src/lib/tauri.ts src/lib/mock.test.ts
git commit -m "feat(wiring): pickFolder + cloneRepo on LoreApi; real listRepos over invoke"
```

---

## Task 6: Frontend — wire the repo picker (open folder + clone)

**Files:**
- Modify: `src/lib/RepoPicker.svelte`

- [ ] **Step 1: Rewrite the picker**

Replace the entire contents of `src/lib/RepoPicker.svelte` with:

```svelte
<script lang="ts">
  import { api } from './api'
  import { session, selectRepo } from './session.svelte'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'
  import type { RepoEntry } from './types'

  let repos = $state<RepoEntry[]>([])
  let loading = $state(false)
  // '' | 'open' | `clone:<name>` — drives the in-flight button labels.
  let busy = $state('')

  async function loadRepos() {
    loading = true
    try {
      repos = await api.listRepos(session.config.serverUrl!)
    } catch (e) {
      toastError("Couldn't list repositories", e)
    } finally {
      loading = false
    }
  }

  async function openFolder() {
    const path = await api.pickFolder()
    if (!path) return // cancelled
    busy = 'open'
    try {
      await api.getStatus(path) // validates it is a Lore working copy
      await selectRepo(path)
    } catch (e) {
      toastError('Not a Lore repository', e)
    } finally {
      busy = ''
    }
  }

  async function cloneRepo(entry: RepoEntry) {
    const parent = await api.pickFolder()
    if (!parent) return // cancelled
    busy = `clone:${entry.name}`
    try {
      const path = await api.cloneRepo(session.config.serverUrl!, entry.id, entry.name, parent)
      await selectRepo(path)
    } catch (e) {
      toastError('Clone failed', e)
    } finally {
      busy = ''
    }
  }

  $effect(() => { loadRepos() })
</script>

<div class="picker">
 <div class="inner">
  <h2>Open a repository</h2>

  <div class="card">
    <div><strong>Local working copy</strong><p class="muted small">Choose a folder you've already cloned.</p></div>
    <span class="spacer"></span>
    <button class="accent" onclick={openFolder} disabled={busy === 'open'}>
      {busy === 'open' ? 'Opening…' : 'Open folder…'}
    </button>
  </div>

  <h3>On {session.config.serverUrl}</h3>
  {#if loading}<p class="muted">Loading repositories…</p>{/if}
  <ul class="repos">
    {#each repos as r (r.id)}
      <li>
        <span class="ico"><Icon name="folder" size={16} /></span>
        <div class="meta"><strong>{r.name}</strong><p class="muted small mono">{r.id.slice(0, 12)}…</p></div>
        <span class="spacer"></span>
        <button onclick={() => cloneRepo(r)} disabled={busy === `clone:${r.name}`}>
          {busy === `clone:${r.name}` ? 'Cloning…' : 'Clone…'}
        </button>
      </li>
    {/each}
  </ul>
 </div>
</div>

<style>
  .picker { flex: 1; overflow: auto; }
  .inner { max-width: 620px; margin: 6vh auto; padding: 0 20px; }
  h2 { font-size: 18px; font-weight: 500; margin: 0 0 16px; }
  h3 { margin: 22px 0 8px; font-size: 12px; color: var(--text-muted); font-weight: 500; }
  .card { display: flex; align-items: center; gap: 12px; border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 14px 16px; background: var(--panel); }
  .small { font-size: 12px; margin: 2px 0 0; }
  .mono { font-family: var(--font-mono); }
  .repos { list-style: none; padding: 0; margin: 0; }
  .repos li { display: flex; align-items: center; gap: 10px; padding: 10px 4px; border-bottom: 1px solid var(--border); }
  .ico { color: var(--accent); }
  .meta { min-width: 0; }
</style>
```

- [ ] **Step 2: Typecheck**

Run: `npm run check`
Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Run the existing tests (nothing regressed)**

Run: `npm test`
Expected: all suites PASS.

- [ ] **Step 4: Commit**

```bash
git add src/lib/RepoPicker.svelte
git commit -m "feat(wiring): real repo picker — open local working copy + clone server repo"
```

---

## Task 7: Frontend — route existing call-site errors to toasts

**Files:**
- Modify: `src/lib/repo.svelte.ts`
- Modify: `src/lib/session.svelte.ts`
- Modify: `src/lib/SignIn.svelte`
- Modify: `src/lib/Changes.svelte`

- [ ] **Step 1: Reroute `repo.svelte.ts`**

Replace the entire contents of `src/lib/repo.svelte.ts` with:

```ts
import { api } from './api'
import { session } from './session.svelte'
import { toastError } from './toast'
import type { LockEntry, StatusResult } from './types'

// The current repository's status + in-flight action, shared by the title bar
// (branch, ahead/behind, sync, push) and the Changes view (files, commit).
export const repo = $state({
  status: null as StatusResult | null,
  busy: '' as '' | 'status' | 'commit' | 'push' | 'sync',
})

export const locks = $state({ list: [] as LockEntry[] })

export async function refreshLocks() {
  const path = session.config.currentRepo
  if (!path) { locks.list = []; return }
  try { locks.list = await api.getLocks(path) }
  catch (e) { toastError("Couldn't load locks", e) }
}

export async function refreshStatus() {
  const path = session.config.currentRepo
  if (!path) { repo.status = null; return }
  repo.busy = 'status'
  try { repo.status = await api.getStatus(path) }
  catch (e) { toastError("Couldn't load changes", e) }
  finally { repo.busy = '' }
}

async function act(kind: 'commit' | 'push' | 'sync', run: (path: string) => Promise<void>) {
  const path = session.config.currentRepo
  if (!path) return
  repo.busy = kind
  try { await run(path) }
  catch (e) {
    toastError(`${kind[0].toUpperCase()}${kind.slice(1)} failed`, e)
    repo.busy = ''
    return
  }
  await refreshStatus()
}

export const commit = (message: string) => act('commit', (p) => api.commitAll(p, message))
export const push = () => act('push', (p) => api.push(p))
export const sync = () => act('sync', (p) => api.sync(p))

export async function setLock(path: string, lock: boolean) {
  const p = session.config.currentRepo
  if (!p) return
  try { await api.setLock(p, path, lock) }
  catch (e) { toastError(lock ? 'Lock failed' : 'Unlock failed', e); return }
  await refreshStatus()
  await refreshLocks()
}
```

- [ ] **Step 2: Guard `bootstrap` in `session.svelte.ts`**

In `src/lib/session.svelte.ts`, add the toast import at the top (after the existing imports):

```ts
import { toastError } from './toast'
```

and replace the `bootstrap` function with:

```ts
export async function bootstrap() {
  try {
    session.config = await api.loadConfig()
    session.signedIn = await api.isAuthenticated()
  } catch (e) {
    toastError('Startup failed', e)
  } finally {
    session.ready = true
  }
}
```

- [ ] **Step 3: Reroute the sign-in failure in `SignIn.svelte`**

In `src/lib/SignIn.svelte`, add the toast import after the existing imports in the `<script>`:

```ts
  import { toastError } from './toast'
```

and change the `catch` clause in `go()` from `catch (e) { error = String(e) }` to:

```ts
    } catch (e) { toastError('Sign-in failed', e) } finally { busy = false }
```

(The inline `error` state stays — it still shows the URL-format validation message.)

- [ ] **Step 4: Remove the dead inline error in `Changes.svelte`**

In `src/lib/Changes.svelte`, delete this line (currently line 38):

```svelte
  {#if repo.error}<p class="error pad">{repo.error}</p>{/if}
```

- [ ] **Step 5: Typecheck**

Run: `npm run check`
Expected: 0 errors, 0 warnings. (If `svelte-check` flags an unused `.error.pad` CSS selector in `Changes.svelte`, delete that selector too.)

- [ ] **Step 6: Run the tests**

Run: `npm test`
Expected: all suites PASS.

- [ ] **Step 7: Commit**

```bash
git add src/lib/repo.svelte.ts src/lib/session.svelte.ts src/lib/SignIn.svelte src/lib/Changes.svelte
git commit -m "feat(wiring): route status/commit/push/sync/lock/sign-in errors to toasts"
```

---

## Task 8: End-to-end verification in `tauri dev` (user-assisted)

**Goal:** prove the real picker against the live server. This task is interactive — the controller drives `tauri dev` and asks the user to confirm each step (the folder/clone dialogs and browser SSO need a human). No code changes unless a defect surfaces.

**Preconditions:**
- `lore` is on `PATH` (`lore --version` → `0.8.3+201`).
- A valid identity exists (`lore auth list --json` shows a future `expires`); if not, run `lore login lore://lore.example.com:41337`.
- The local working copy `C:/Users/jimmy/lore-test-repo` (repo `desktoptest1`) exists.

- [ ] **Step 1: Launch the app**

Run: `npx tauri dev` (ensure port 5173 is free first). Wait for the window.

- [ ] **Step 2: Sign in (if needed)**

If the app shows the sign-in screen, complete browser SSO. Confirm it lands on the repo picker.

- [ ] **Step 3: Verify the real repository list**

Confirm the "On lore://lore.example.com:41337" list shows the real repos (`desktoptest1`, `ssotest11`, `ssotest10`) with truncated ids — not the mock's `game-main` / `game-assets` / `audio`.

- [ ] **Step 4: Open a local working copy**

Click "Open folder…", pick `C:/Users/jimmy/lore-test-repo`. Confirm the app enters the repo and the Changes/History views show real data (real branch, real commits).

- [ ] **Step 5: Verify the error toast**

Return to the picker (TitleBar "Current repository" button). Click "Open folder…" and pick a folder that is **not** a Lore working copy (e.g. `C:/Windows`). Confirm a red toast titled "Not a Lore repository" appears, auto-dismisses after a few seconds, and the app stays on the picker.

- [ ] **Step 6: Clone a server repository**

Back on the picker, click "Clone…" on a small server repo, pick an empty destination parent folder (e.g. `C:/Users/jimmy/lore-clones`), and confirm the button shows "Cloning…", the clone completes, and the app lands in the freshly cloned repo with real status/history.

- [ ] **Step 7: Close the app and capture the result**

Stop `tauri dev`. Record the outcome (screenshots or a short note) in the final review.

---

## Self-Review Notes

- **Spec coverage:** `listRepos` (Task 1), `pickFolder` + open-folder validation (Tasks 3, 5, 6), `cloneRepo` (Tasks 2, 3, 5, 6), dialog plugin + capability (Task 3), app-wide error toasts (Tasks 4, 6, 7), English UI text (Tasks 4, 6), tests including the toast store (Task 4) and the clone-args helper (Task 2), E2E (Task 8). All spec sections map to a task.
- **Open items resolved here:** `repositoryListEntry.id` is a hex string (captured fixture, Task 1); `lore clone` is a confirmed top-level command with the URL form `<serverUrl>/<id>` (Task 2); the capability file already exists and only needs `dialog:allow-open` appended (Task 3).
- **Type consistency:** `RepoEntry { id, name }` (types.ts) ↔ `RepoEntryDto { id, name }` (Rust, camelCase); `cloneRepo(serverUrl, repoId, repoName, destParent)` identical across `LoreApi`, mock, tauri, and the `lore_clone` command args; `toastError(title, err?) → number` and `dismissToast(id)` used consistently by the store, Toaster, picker, and rerouted call sites.
