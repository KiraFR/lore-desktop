# Lore Desktop Slice C — Config Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (small change, inline). Steps use `- [ ]`.

**Goal:** Persist `AppConfig` to `<app_config_dir>/config.json` via two typed Rust commands, replacing the localStorage-only fallback in the real Tauri app.

**Architecture:** New `src-tauri/src/config.rs` with pure path-based helpers (`load_config_from`/`save_config_to`) + thin `config_load`/`config_save` commands that resolve the app config dir; registered in `lib.rs`; `tauri.ts` overrides `loadConfig`/`saveConfig` with `invoke`. Mock (localStorage) unchanged for browser dev.

**Tech Stack:** Tauri v2 (Rust, `tauri::Manager` path API), serde/serde_json, Svelte/TS. No new deps or capabilities (Rust-side `std::fs` + `app.path()`).

**Branch:** `wiring-slice-c-config`. Repo root: `C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop`.

---

## Task 1: `config.rs` (DTO + helpers + commands + tests)

**Files:** Create `src-tauri/src/config.rs`.

- [ ] **Step 1: Write the whole module (helpers, commands, TDD tests)**

```rust
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Persisted app config; mirrors the TS `AppConfig`.
#[derive(Serialize, Deserialize, Default, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigDto {
    pub server_url: Option<String>,
    pub current_repo: Option<String>,
    #[serde(default)]
    pub recent_repos: Vec<String>,
}

/// Read + deserialize the config file. A missing file or any read/parse error
/// yields defaults — this never errors, so the app always boots.
pub fn load_config_from(path: &Path) -> AppConfigDto {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Serialize + write the config atomically (temp file then rename), creating the
/// parent directory if needed.
pub fn save_config_to(path: &Path, cfg: &AppConfigDto) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("creating config dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("serializing config: {e}"))?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("writing config: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("finalizing config: {e}"))?;
    Ok(())
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|d| d.join("config.json"))
        .map_err(|e| format!("resolving config dir: {e}"))
}

#[tauri::command]
pub fn config_load(app: tauri::AppHandle) -> AppConfigDto {
    match config_path(&app) {
        Ok(p) => load_config_from(&p),
        Err(_) => AppConfigDto::default(),
    }
}

#[tauri::command]
pub fn config_save(app: tauri::AppHandle, config: AppConfigDto) -> Result<(), String> {
    let p = config_path(&app)?;
    save_config_to(&p, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("lore-desktop-cfgtest-{name}"))
    }

    #[test]
    fn round_trip() {
        let d = dir("roundtrip");
        let _ = std::fs::remove_dir_all(&d);
        let path = d.join("config.json");
        let cfg = AppConfigDto {
            server_url: Some("lore://host:41337".into()),
            current_repo: Some("C:/repos/game".into()),
            recent_repos: vec!["C:/repos/game".into(), "C:/repos/x".into()],
        };
        save_config_to(&path, &cfg).unwrap();
        assert_eq!(load_config_from(&path), cfg);
    }

    #[test]
    fn missing_file_is_default() {
        let path = dir("missing").join("nope.json");
        let _ = std::fs::remove_dir_all(dir("missing"));
        assert_eq!(load_config_from(&path), AppConfigDto::default());
    }

    #[test]
    fn corrupt_file_is_default() {
        let d = dir("corrupt");
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join("config.json");
        std::fs::write(&path, b"not json {{{").unwrap();
        assert_eq!(load_config_from(&path), AppConfigDto::default());
    }
}
```

- [ ] **Step 2: Register the module + commands in `src-tauri/src/lib.rs`** — add `mod config;` next to `mod commands;`, and add `config::config_load, config::config_save,` to the `tauri::generate_handler![…]` list (after `commands::lore_clone,`).

- [ ] **Step 3: Run the tests** — `cargo test --manifest-path src-tauri/Cargo.toml config` → expect the 3 `config::tests` pass.

- [ ] **Step 4: Build** — `cargo build --manifest-path src-tauri/Cargo.toml` → clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat(wiring): persist AppConfig to <app_config_dir>/config.json (Rust commands)"
```

## Task 2: Wire `tauri.ts`

**Files:** Modify `src/lib/tauri.ts`.

- [ ] **Step 1: Add the two overrides** — import `AppConfig` in the type import and add to the `tauriApi` object (after `cloneRepo`):

```ts
import type { AppConfig, HistoryPage, LoreApi, RepoEntry, StatusResult } from './types'
```
```ts
  loadConfig: () => invoke<AppConfig>('config_load'),
  saveConfig: (config) => invoke<void>('config_save', { config }),
```

- [ ] **Step 2: Typecheck + tests** — `npm run check` (0/0) and `npm test` (all pass; mock/config tests unchanged).

- [ ] **Step 3: Commit**

```bash
git add src/lib/tauri.ts
git commit -m "feat(wiring): loadConfig/saveConfig over invoke (real on-disk config)"
```

## Task 3: Merge

- [ ] Merge `wiring-slice-c-config` → `main` + push (per user go). Config now persists to disk; confirmed opportunistically on the next `tauri dev` run (serverUrl/currentRepo survive a full app restart).
