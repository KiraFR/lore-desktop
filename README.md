# Lore Desktop

A desktop client for the [Lore](https://github.com/EpicGames/lore) VCS — sign in to a Lore server (+ SSO), pick a repository, review changes, commit, push, sync, and resolve merges without touching the CLI. Windows-first.

## Install

1. Download the latest `Lore.Desktop_x.y.z_x64-setup.exe` from the [releases page](https://github.com/KiraFR/lore-desktop/releases/latest) and run it.
2. **Once per machine**, trust the studio's code-signing certificate so Windows and app-control tools accept the app and its updates (imports a public certificate only) — from an elevated PowerShell in a checkout or download of this repo:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts\trust-studio-cert.ps1
   ```

3. Make sure the [`lore`](https://github.com/EpicGames/lore) CLI is installed and on your `PATH` — the app drives it for every operation.
4. Launch the app, enter your team's Lore server URL (`lore://host:41337`), and finish sign-in in the browser (SSO).

The app keeps itself up to date: when a new version is released, a banner offers **Install & restart** — one click and you're current. You can also check from **Preferences ▸ Support**.

Requirements: Windows 10/11 with WebView2 (preinstalled on Windows 11), and network access to your Lore server.

## What it does

**Auth & repositories**
- Sign in to a Lore server (browser-based login + SSO, `lore login`); sign out.
- Repository picker/switcher: browse the server's repo list, clone one, or "Add existing repository…" to register a local working copy already on disk.
- Detects a repo folder that has moved or gone missing and lets you relocate it in place.
- Optional shared object store (enable/disable "use automatically for clones" for the current server).

**Changes**
- Working-tree file list: add / modify / delete / move / copy, binary hints, asset size deltas (old → new), a locked-by-teammate section.
- `.loreignore` support: files excluded by the repo's ignore rules stay out of the list ("N ignored" in the summary).
- Multi-select with keyboard navigation, bulk stage/unstage/lock/discard from the context menu.
- Discard a file's working changes; per-file preview pane.
- Commit selected files with a message; amend the last (unpushed) commit; undo the last local commit (its changes return to pending, with a warning when pending edits would be overwritten).

**Push / Sync**
- Push, with progress reporting; on a non-fast-forward refusal (remote diverged), a "Sync & push" recovery action chains sync then push.
- After pushing files you had locked, a "Release locks" prompt.
- Sync (non-destructive pull/merge of the remote); time-travel the working copy to any past revision.

**History**
- Commit graph with per-branch lanes, paginated loading, and a client-side search box (message / author / short hash / revision number).
- Per-file history and preview from the History view; restore a file to a past version.

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
- Real-time notifications (teammate pushes and lock changes) surfaced as toasts.
- "About repository" panel (id, remote URL, description, default branch, created date).
- Light and dark themes.

## Develop

```bash
npm install
npm run dev        # browser design loop — http://localhost:5173, driven by an in-memory mock, no Rust needed
npm run tauri dev  # full app against a real `lore` CLI (needs the Rust toolchain + `lore` on PATH)
npm test           # vitest
npm run check      # svelte-check + tsc
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

To release: bump the version in `src-tauri/tauri.conf.json`, tag `vX.Y.Z`, push the tag — CI builds, signs, and publishes the GitHub release that installed apps pick up automatically (see [scripts/README.md](scripts/README.md)).
