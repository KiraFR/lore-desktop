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
        let _ = std::fs::remove_dir_all(dir("missing"));
        let path = dir("missing").join("nope.json");
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
