# Lore Desktop — wiring Slice B design (repo picker: list, open, clone)

- **Date:** 2026-07-05
- **Repo:** github.com/KiraFR/lore-desktop
- **Status:** design approved; next = writing-plans → subagent-driven implementation
- **Builds on:** Slice A (`2026-07-05-lore-desktop-wiring-design.md`). The mock→real pipeline — Tauri command → `lore … --json` → Rust NDJSON parse → typed DTO → `LoreApi` — is in place and proven end-to-end. Slice B extends that pipeline to the repository picker.

## Goal

Make the repository picker real: list a server's repositories, open an already-cloned local working copy through a native folder dialog, and clone a selected server repository to a chosen local location. This retires the last mock on the entry path and the E2E temp-seed hack — the app now reaches a real working copy entirely through the UI.

## Scope

Three `LoreApi` capabilities change or appear:

- `listRepos(serverUrl)` — becomes real, replacing the mock's fake repositories.
- `pickFolder()` — new; a native OS directory chooser, used both to open a local repo and to choose a clone destination.
- `cloneRepo(serverUrl, repoId, repoName, destParent)` — new; clone a server repo into `<destParent>/<repoName>` and return the created path.

`getStatus` (wired in Slice A) doubles as the validity check when opening a folder.

## Architecture (unchanged from Slice A)

Every host-touching operation is a typed Rust `#[tauri::command]` in `src-tauri` that shells `lore … --json`, parses the NDJSON stream with `serde`, and returns a struct serializing to the exact shape the TypeScript `LoreApi` expects. `src/lib/api.ts` swaps `mock` ↔ `tauriApi` by detecting `__TAURI_INTERNALS__`. Components stay backend-agnostic; only `tauri.ts`, `mock.ts`, the `LoreApi` interface, and the picker component change.

The native folder dialog is the one new host capability. It is reached through `@tauri-apps/plugin-dialog` from `tauri.ts` (not a bespoke Rust command) and exposed to components as `LoreApi.pickFolder()`, so the mock/browser path keeps working.

## Command mapping

| `LoreApi` method | `lore` invocation | Result |
|---|---|---|
| `listRepos(serverUrl)` | `lore repository list <serverUrl> --json` | `repositoryListEntry` events → `{ id, name }[]` |
| `pickFolder()` | *(native dialog `open({ directory: true })`)* | the chosen absolute path, or `null` if cancelled |
| `cloneRepo(serverUrl, repoId, repoName, destParent)` | `lore clone <serverUrl>/<repoId> <destParent>/<repoName> --json` | success = exit 0 + terminal `complete.status == 0`; returns the created path |

## NDJSON parsing (reuse)

`lore_repositories` reuses the Slice A `lore` module (`run_lore`, `parse_events`, `events_with_tag`, `check_ok`). A `repositoryListEntry` event carries `id` (a repository id) and `name`; both map straight onto `RepoEntry`. `id` is rendered in the picker (truncated) and used to build the clone URL.

## Opening a local working copy

A Lore working copy is a directory containing a `.lore/` marker (legacy `.urc/` is also accepted). `lore` detects the format and requires the marker on every `--repository` call, erroring `no lore repository at <path> (missing .lore)` when it is absent (`lore/src/storage/open.rs`, `lore/src/call.rs`).

`pickFolder()` returns a directory; the picker then calls `getStatus(path)`. Success ⇒ a valid working copy ⇒ `selectRepo(path)`. Failure ⇒ the error is shown inline in the picker and the current repo is left unchanged. No separate validation command and no instance registration are needed — reading through `--repository <path>` is sufficient, and this is the exact path Slice A's E2E already exercised.

## Cloning

Selecting a repository in the list starts a clone:

1. `pickFolder()` chooses the destination **parent** directory (cancel aborts).
2. `cloneRepo(serverUrl, repoId, repoName, destParent)` runs `lore clone <serverUrl>/<repoId> <path> --json`, where `<path>` is `destParent` joined with `repoName`, built in Rust with `PathBuf` for cross-platform correctness. The clone URL addresses the repository by id.
3. The command blocks until the clone finishes, checks success (process exit code and terminal `complete.status`), and returns the created path as a string.
4. The picker calls `selectRepo(returnedPath)`, landing in the fresh clone.

Clone runs blocking with an indeterminate "Cloning <name>…" state in the picker. A clone whose destination already exists and is non-empty fails inside `lore`; that error surfaces inline. Clone reads from the server and writes only to the local destination — it never mutates server state.

## Native folder dialog

`@tauri-apps/plugin-dialog` (JS) and `tauri-plugin-dialog` (Rust) are added; the Rust plugin is registered in `lib.rs` and the capabilities file grants `dialog:allow-open`. `tauri.ts`'s `pickFolder` calls `open({ directory: true, multiple: false })`, returning `string | null`. The mock returns a canned path so browser-only dev of the picker keeps working.

## Component (`RepoPicker.svelte`)

- The server list already renders from `api.listRepos`; it now shows real data. Its loading and error states already exist.
- The "Open folder" button calls `openFolder()`: `pickFolder()` → on a path, validate with `getStatus` → `selectRepo`, else inline error.
- Each list row's button becomes a real "Clone…": pick a parent with `pickFolder()` → set a "Cloning…" busy state → `api.cloneRepo(...)` → `selectRepo(finalPath)`; errors inline; busy cleared in `finally`.
- A single `busy`/`error` pair covers open + clone. The existing escape hatch (the TitleBar "Current repository" button → `clearCurrentRepo`) returns to the picker.

## Error handling

`listRepos` / `cloneRepo` failures (auth expired, network, bad URL, non-empty destination) reject from `invoke` and are caught in the picker, shown inline via its existing `error` state. `pickFolder()` returning `null` (cancel) is a no-op. Opening a non-Lore folder surfaces the `getStatus` error inline. No new global error UI is added.

## Testing

1. **Rust unit test:** `repositories_from` against a captured `repo_list.ndjson` fixture (regenerate with `lore repository list <serverUrl> --json`).
2. **Rust unit test:** `build_clone_args(serverUrl, id, name, parent) -> (url, path)` pure helper — URL is `serverUrl/id`, path is `parent` joined with `name`. The clone's real network/FS behavior is covered by E2E, not a unit test.
3. **Frontend:** mock `pickFolder` (canned path) and `cloneRepo` (returns a fake path) keep the picker exercisable in vitest and the browser.
4. **E2E in `tauri dev`:** sign in → the picker lists the account's real server repos; "Open folder" → pick the local `lore-test-repo` → land in it with real status/history; select a server repo → pick a parent → clone completes → land in the fresh clone.

## Open items to resolve during implementation

- Exact JSON serialization of `repositoryListEntry.id` (repository id → hex string) — read off a captured fixture.
- Clone URL form `<serverUrl>/<repoId>` accepted by `lore clone` (already used successfully to clone `desktoptest1` by id; reconfirm at E2E).
- Clone `--json` completion detection: terminal `complete.status` plus process exit code; whether a `repositoryCloneEnd` event is also required to call it success.
- Tauri v2 dialog plugin permission wiring (`dialog:allow-open`) in the capabilities file.

## Out of scope (later slices)

Live clone progress bar (streaming `repositoryCloneProgress` via Tauri channels), a "recent repositories" section in the picker, branch/lock wiring, and server-mutating writes (commit / push / sync / merge / setLock). Every not-yet-wired method keeps using the mock via `api.ts`.
