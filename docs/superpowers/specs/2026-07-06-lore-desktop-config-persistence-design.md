# Lore Desktop — Slice C: config persistence (Tauri fs)

- **Date:** 2026-07-06
- **Repo:** github.com/KiraFR/lore-desktop, branch `wiring-slice-c-config`
- **Ticket:** TICKET-127 (Lore Desktop)
- **Status:** design approved; next = writing-plans → inline implementation
- **Builds on:** Slice A + Slice B (the mock→real Tauri-command pipeline; `api.ts` picks `tauriApi` inside Tauri else `mock`).

## Problem

`src/lib/tauri.ts` spreads `...mock` and does **not** override `loadConfig` / `saveConfig`, so in the real Tauri app the app config (`serverUrl`, `currentRepo`, `recentRepos`) persists only in the WebView2 `localStorage`. That is fragile — it is lost if the webview data dir resets, which is the root of the Slice B "serverUrl missing" bug — and it is not a real on-disk config file.

## Goal

Persist `AppConfig` to a JSON file in the app's config directory via typed Rust commands, so config survives app restarts and webview resets. The read side never fails: a missing or corrupt file yields defaults, so the app always boots.

## Architecture (unchanged pattern)

Two Rust `#[tauri::command]`s in `src-tauri`, wired into `tauri.ts`. The mock (localStorage) stays for browser dev. `api.ts`'s mock↔tauri swap is unchanged. Components are untouched — `loadConfig` / `saveConfig` keep the same `LoreApi` signatures.

## Design

- **File:** `<app_config_dir>/config.json`, where `<app_config_dir>` = `app.path().app_config_dir()` (Tauri v2; identifier `com.tauri.dev` → e.g. `%APPDATA%/com.tauri.dev/config.json` on Windows).
- **DTO** (`src-tauri/src/config.rs`): `AppConfigDto { server_url: Option<String>, current_repo: Option<String>, recent_repos: Vec<String> }` with `#[serde(rename_all = "camelCase")]`, `Serialize`, `Deserialize`, `Default` → matches the TS `AppConfig { serverUrl, currentRepo, recentRepos }`.
- **Pure, path-based helpers** (unit-testable, no `AppHandle`):
  - `load_config_from(path: &Path) -> AppConfigDto` — read + deserialize the file; a missing file OR any read/parse error returns `AppConfigDto::default()` (`{ server_url: None, current_repo: None, recent_repos: [] }`). Never errors.
  - `save_config_to(path: &Path, cfg: &AppConfigDto) -> Result<(), String>` — create the parent dir (`create_dir_all`); serialize `cfg` to pretty JSON; **write atomically** — write to `<path>.tmp` then `rename` over `<path>`.
- **Commands** (`config.rs`, registered in `lib.rs`):
  - `config_load(app: tauri::AppHandle) -> AppConfigDto` — resolve `app_config_dir()/config.json` (`use tauri::Manager`); on a dir-resolve failure, return `AppConfigDto::default()`; otherwise `load_config_from`. Never errors.
  - `config_save(app: tauri::AppHandle, config: AppConfigDto) -> Result<(), String>` — resolve the path; on dir-resolve failure return `Err`; otherwise `save_config_to`.
- **`tauri.ts`** — add two overrides: `loadConfig: () => invoke('config_load')` and `saveConfig: (config) => invoke('config_save', { config })`.
- **`mock.ts`** — unchanged (localStorage; browser `npm run dev` / vitest keep using it).

## No migration

On the first real-app launch after this ships, `config.json` does not exist → defaults. `serverUrl` re-defaults to `DEFAULT_SERVER_URL` (Slice B's `bootstrap`), so a connected user still lands straight on the picker; `recentRepos` starts empty. The dev-only `localStorage` config is intentionally not migrated.

## Error handling

- **Load never throws.** Any failure (missing file, unreadable, malformed JSON, dir-resolve error) returns defaults, so `bootstrap` always completes and the app boots.
- **Save may return `Err`.** The existing callers (`setSignedIn`, `selectRepo`, `clearCurrentRepo`) `await api.saveConfig(...)`; a rejection propagates to their callers' `catch` → the existing `toastError` path. A failed save is non-fatal (config just isn't persisted this time).

## Testing

Rust unit tests on the pure helpers, using a temp-dir file path:
1. **Round-trip:** `save_config_to` a config with a non-null `server_url` + `current_repo` + a `recent_repos` list, then `load_config_from` returns an equal `AppConfigDto`.
2. **Missing file** → `load_config_from` returns `AppConfigDto::default()`.
3. **Corrupt file** (write non-JSON bytes) → `load_config_from` returns `AppConfigDto::default()`.

Frontend is unchanged (`tauri.ts` is a thin `invoke`; the mock's config round-trip test already exists). The end-to-end "config survives a restart" is confirmed opportunistically when the app is next run in `tauri dev`, not gated on a dedicated E2E.

## Out of scope

Migrating existing `localStorage` config; a settings/preferences UI; config schema versioning; storing anything beyond `AppConfig`.
