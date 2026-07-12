# Lore Desktop

A GitHub-Desktop-style client for the [Lore](https://github.com/EpicGames/lore) VCS — sign in to a Lore server (+ SSO), pick a repository, review changes, commit, push, sync, and resolve merges without touching the CLI. Built with **Tauri v2 + Svelte 5 (runes) + TypeScript**. Windows-first.

The app is fully wired to a real `lore` binary: every action shells out to `lore … --json`, parses the NDJSON event stream, and drives the UI from it — there is no mock data path in production.

## What it does

**Auth & repositories**
- Sign in to a Lore server (browser-based login + SSO, `lore login`); sign out.
- Repository picker/switcher: browse the server's repo list, clone one, or "Add existing repository…" to register a local working copy already on disk.
- Detects a repo folder that has moved or gone missing and lets you relocate it in place.
- Optional shared object store (enable/disable "use automatically for clones" for the current server).

**Changes**
- Working-tree file list: add / modify / delete / move / copy, binary hints, asset size deltas (old → new), a locked-by-teammate section.
- Discard a file's working changes; per-file preview pane.
- Commit selected files with a message; amend the last (unpushed) commit; undo the last local commit (its changes return to pending).

**Push / Sync**
- Push, with progress reporting; on a non-fast-forward refusal (remote diverged), a "Sync & push" recovery action chains sync then push.
- After pushing files you had locked, a "Release locks" prompt.
- Sync (non-destructive pull/merge of the remote); time-travel the working copy to any past revision.

**History**
- Commit graph with per-branch lanes, paginated loading, and a client-side search box (message / author / short hash / revision number).
- Per-file history and preview from the History view.

**Branches**
- Local and remote-only sections, switch, create (based on any revision), archive.
- Ahead/behind counters on the current branch.

**Merge**
- Preview an incoming merge (files touched, conflict count) before starting it.
- Conflict resolution with mine/theirs per file, including real working-copy thumbnails for binary conflicts (image thumbnail for "mine", the `~theirs` sidecar for the incoming version).
- Abort or commit once all conflicts are resolved.

**Locks**
- List and filter file locks across the repo (who holds what, when).

**Rich previews**
Working-copy previews render inline instead of showing a generic icon:
- Images: PNG/JPEG/WebP/BMP/GIF/TGA/TIFF, plus DDS (BC1–BC7), EXR/HDR (tone-mapped), PSD (flattened), Blender `.blend` (embedded thumbnail).
- Unreal `.uasset` / `.umap`: the editor-embedded thumbnail, decoded from the package's thumbnail table.
- Audio (wav/ogg/mp3/flac) with waveform peaks and playback.
- 3D models (glTF/GLB, OBJ, FBX) via an orbiting three.js viewer.

**Other**
- Real-time notifications (a background `lore notification subscribe` sidecar) for teammate pushes and lock changes, surfaced as toasts.
- "About repository" panel (id, remote URL, description, default branch, created date).
- Light and dark themes.

## Architecture

- **`src/lib/types.ts`** — the `LoreApi` contract: every data operation the UI can perform, plus the shared types (`StatusResult`, `ChangedFile`, `Commit`, `Branch`, `MergeConflict`, `PreviewData`, `AppConfig`, …).
- **`src/lib/api.ts`** — the app's single data boundary. At runtime it picks one implementation based on whether a Tauri context is present:
  - **`src/lib/mock.ts`** — a stateful in-memory/`localStorage` fake implementing the exact same `LoreApi` contract, used only for the browser design loop (`npm run dev`).
  - **`src/lib/tauri.ts`** — the real implementation, calling `@tauri-apps/api`'s `invoke` for each command.
- **Svelte 5 rune stores** (`src/lib/*.svelte.ts`, e.g. `session.svelte.ts`, `repo.svelte.ts`, `ui.svelte.ts`, `opProgress.svelte.ts`, `repoHealth.svelte.ts`, `notifications.svelte.ts`, `thumbs.svelte.ts`) hold reactive app state; components in `src/lib/*.svelte` (`Changes`, `History`, `Merge`, `Locks`, `RepoSwitcher`, `TitleBar`, `AboutRepo`, …) consume them, wired together by `src/App.svelte`.
- **Pure TS logic modules** (no Svelte, no Tauri) are split out wherever practical — `pushErrors.ts`, `mergeLogic.ts`, `historyFilter.ts`, `branchGrouping.ts`, `sizeFormat.ts`, `fileTypes.ts`, etc. — each with a `*.test.ts` next to it, so business logic is unit-testable under vitest without a Tauri or DOM runtime.
- **Rust side** (`src-tauri/src/`):
  - `commands.rs` — the `#[tauri::command]` entry points; each shells `lore <subcommand> --json` (or a streaming variant for long ops like clone/push/sync) and maps the CLI's NDJSON events into the DTOs the frontend expects.
  - `lore.rs` — the process runner: spawns `lore`, parses its NDJSON stream, exposes both a one-shot and a streaming (progress-emitting) mode.
  - `config.rs` — persists `AppConfig` (server URL, current/recent repos, display name) to disk.
  - `notifications.rs` — manages a background `lore notification subscribe` child process and forwards its events to the webview.
  - `preview.rs` — the working-copy preview pipeline: format decoding (DDS/PSD/EXR/HDR/Blender/`.uasset`/`.umap`/`.sbsar`), thumbnail generation, and an on-disk cache keyed by path + mtime + size.

## Develop

```bash
npm install
npm run dev        # browser design loop — http://localhost:5173, driven by the in-memory mock, no Rust needed
npm run tauri dev  # full app against a real `lore` CLI (needs the Rust toolchain + `lore` on PATH)
npm test           # vitest — pure-logic and mock/contract tests
npm run check      # svelte-check + tsc
npm run build      # production web build
```

`npm run dev` is the fast design-iteration loop: edit any `.svelte` file or `src/app.css` and see it live against the mock, with zero backend. The design system lives in `src/app.css` — CSS-variable tokens for color, spacing, and radius, with both light and dark palettes.

`npm run tauri dev` runs the same UI inside the Tauri shell, wired to the real `lore` CLI — use this when a change touches `src-tauri/` or the actual repo behavior, not just layout/styling.

## Requirements

- The [`lore`](https://github.com/EpicGames/lore) CLI installed and on `PATH` — required for `npm run tauri dev` and any production build; the browser design loop (`npm run dev`) does not need it.
- Rust toolchain, for building/running the Tauri shell.
- Windows with WebView2 (the primary target platform).

## The mock

`src/lib/mock.ts` implements the exact `LoreApi` interface the real backend implements, so the browser design loop is a faithful stand-in: the same components, the same data shapes, the same state transitions — just backed by memory instead of a real repository.
