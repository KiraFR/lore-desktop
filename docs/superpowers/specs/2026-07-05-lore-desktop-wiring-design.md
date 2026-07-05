# Lore Desktop — wiring Slice A design (mock → real `lore`, read-only)

- **Date:** 2026-07-05
- **Repo:** github.com/KiraFR/lore-desktop
- **Status:** design approved; next = writing-plans → subagent-driven implementation
- **Builds on:** `2026-07-04-lore-desktop-design.md` (functional), `2026-07-05-lore-desktop-visual-design.md` (visual). Slice 1 (mock UI) + Slice 2 (all screens, GitHub-Desktop × GitKraken, virtualized) shipped on `main` (commit `808ebeb`). The Tauri backend (`src-tauri`) is scaffolded and confirmed to build + link on this machine.

## Goal

Replace the in-memory mock with a real backend for a **thin, read-only vertical slice**, proving the whole pipeline end-to-end against the live `lore.example.com` server:

> Tauri command → shell `lore … --json` → parse NDJSON in Rust → typed result → `LoreApi` → live UI.

Slice A wires exactly four methods: `isAuthenticated`, `signIn`, `getStatus`, `getHistory`. Writes and the remaining reads follow in later slices once the pipeline is proven.

## Architecture

- Each wired `LoreApi` method is a **typed Rust `#[tauri::command]`** in `src-tauri`. It runs the matching `lore` subcommand with `--json` and (where a repo is involved) `--repository <path>` via `std::process::Command`, parses the NDJSON stream in Rust with `serde`, and returns a struct that serializes to the exact shape the TypeScript `LoreApi` expects.
- The frontend adds `@tauri-apps/api`. `src/lib/api.ts` changes from `export const api: LoreApi = mock` to an object whose methods call `invoke('<command>', …)`. **Every component and the `LoreApi` interface stay unchanged**, except the `getHistory` signature change below, which is applied to the mock too so both stay parallel.
- `src/lib/mock.ts` and its tests remain. `api.ts` is the single swap point: in Slice A it points at the Tauri implementation; the mock stays available for browser-only UI work on the not-yet-wired screens.

## NDJSON parsing (Rust)

`lore … --json` emits one JSON object per line — `{"tagName": "<name>", "data": { … }}` — terminated by `{"tagName": "complete", "data": {"status": <n>}}`.

A `ndjson` module reads stdout line by line, deserializes each line into `{ tagName, data }`, and dispatches by `tagName` into typed accumulators. Success is determined by the terminal `complete.status` together with the process exit code: non-zero → the command returns `Err(message)`, surfaced through the UI's existing `repo.error` path. The parser is unit-tested in Rust against captured sample output.

## Command mapping (Slice A)

| `LoreApi` method | `lore` invocation | Parsed from |
|---|---|---|
| `isAuthenticated()` | `lore auth list --json` | `authIdentity` events; true iff an identity for the signed-in server exists and its `expires` is in the future |
| `signIn(serverUrl, authUrl?)` | `lore login [--auth-url <authUrl>] <serverUrl>` | success = exit 0 (drives browser SSO, stores the token in the OS keychain) |
| `getStatus(repoPath)` | `lore status --repository <repoPath> --json` | `repositoryStatusRevision` (branch, local/remote ahead) + `repositoryStatusFile` (path, action, binary flag) |
| `getHistory(repoPath, length, cursor?)` | `lore history <length> [--revision <cursor>] --repository <repoPath> --json` | revision events → `Commit[]`; next cursor = the last revision id in the page |

Notes:

- `signIn` runs interactively (opens the browser) and blocks until the CLI completes; the sign-in UI already shows a "complete sign-in in your browser…" state.
- `getStatus` runs **without** `--scan` in Slice A: it is read-only, and `--scan`/`--check-dirty` persist dirty flags and require write access — deferred to the writes slice.

## Pagination

`lore history` paginates natively: positional `LENGTH` + `--revision <start>` ("show LENGTH revisions starting at `<start>`"). `getHistory` therefore becomes `getHistory(repoPath, length, cursor?)` and returns `{ commits, nextCursor }`, where `nextCursor` is the last revision id of the page (or `null` at the end). The History component — already virtualized — requests the next page when the scroll window nears the loaded end, appends the results, and advances the cursor. The mock's `getHistory` adopts the same signature so the contract stays single-sourced.

## Graph fidelity (known unknown)

The commit graph's lanes are computed from each commit's `lane` + `parents[]`. Whether `lore history --json` exposes per-revision **parents** and **branch** is unverified — the exact revision-event shape must be read off a real repo during implementation. If they are available, the Rust command maps them into `Commit.parents` / `Commit.lane`; if not, Slice A renders a single-lane (linear) graph and full topology becomes a follow-up. Either way the pipeline proof is unaffected.

## Auth state

A stored identity for `lore.example.com` already exists locally, but its token expired on the morning of 2026-07-05. Live testing re-runs `lore login`. `isAuthenticated` must treat an expired identity as signed-out (compare `expires` against now).

## Error handling

Any command whose terminal `complete.status` is non-zero, or whose process exits non-zero, returns `Err(<summary/stderr>)`. The frontend already renders `repo.error`, so `signIn` / `getStatus` / `getHistory` failures surface there. No new UI is added in Slice A.

## Testing

1. Clone one of the account's server repos locally: `lore clone lore://lore.example.com…`.
2. `lore login` to refresh the token.
3. `npx tauri dev` (confirmed to build + run on this machine: AppLocker inactive, Rust 1.95, WebView2 149, MSVC linker OK).
4. Verify against real data: sign-in succeeds; the status view shows the real branch + ahead/behind + changed files; History shows real commits and paginates on scroll.
5. Rust unit tests for the NDJSON parser (captured sample lines).

## Out of scope (later slices)

Writes (commit / push / sync / merge / `setLock`), the remaining reads (branches, locks, repo list), real binary Before/After thumbnails, and bundling a pinned `lore` binary (Slice A shells the `lore` already on `PATH`). Every not-yet-wired method keeps using the mock via `api.ts` until its slice lands.

## Open items to resolve during implementation

- Exact `lore history --json` revision-event shape (field names, parents, branch), read from a real repo.
- Exact `repositoryStatusRevision` / `repositoryStatusFile` field names for branch, ahead/behind, and the binary flag.
- `signIn` completion detection: block on process exit vs. poll `auth list`.
