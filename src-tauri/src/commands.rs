use crate::lore::{events_with_tag, run_lore, LoreEvent};

#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}

/// True iff any stored identity has an `expires` in the future.
fn is_authenticated_from(events: &[crate::lore::LoreEvent], now_ms: i64) -> bool {
    events_with_tag(events, "authIdentity").iter().any(|d| {
        d.get("expires").and_then(|e| e.as_i64()).map(|exp| exp > now_ms).unwrap_or(false)
    })
}

#[tauri::command]
pub fn lore_is_authenticated() -> Result<bool, String> {
    let events = run_lore(&["auth", "list"])?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(is_authenticated_from(&events, now_ms))
}

use serde::Serialize;

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFileDto {
    pub path: String,
    pub action: String,   // "add" | "modify" | "delete" | "move" | "copy"
    pub is_binary: bool,
    pub size: u64,
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusResultDto {
    pub branch: String,
    pub local_ahead: u64,
    pub remote_ahead: u64,
    pub files: Vec<ChangedFileDto>,
}

const BINARY_EXTS: &[&str] = &["uasset", "umap", "png", "fbx", "wav", "tga", "psd"];
fn is_binary_path(path: &str) -> bool {
    path.rsplit('.').next().map(|e| BINARY_EXTS.contains(&e.to_ascii_lowercase().as_str())).unwrap_or(false)
}

/// Map the `action` value from `repositoryStatusFile` to the UI action string.
/// `keep` is a content modification. (Confirm the wire encoding against the
/// fixture — if `action` is an integer, match on it instead of a string.)
fn map_action(action: &serde_json::Value) -> String {
    match action.as_str() {
        Some("keep") => "modify",
        Some("add") => "add",
        Some("delete") => "delete",
        Some("move") => "move",
        Some("copy") => "copy",
        _ => "modify",
    }
    .to_string()
}

fn status_from(events: &[LoreEvent]) -> StatusResultDto {
    let rev = events_with_tag(events, "repositoryStatusRevision").into_iter().next();
    let branch = rev.and_then(|d| d.get("branchName")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let local_n = rev.and_then(|d| d.get("revisionLocalNumber")).and_then(|v| v.as_u64()).unwrap_or(0);
    let remote_n = rev.and_then(|d| d.get("revisionRemoteNumber")).and_then(|v| v.as_u64()).unwrap_or(0);
    let is_local_ahead = rev.and_then(|d| d.get("isLocalAhead")).map(json_truthy).unwrap_or(false);
    let is_remote_ahead = rev.and_then(|d| d.get("isRemoteAhead")).map(json_truthy).unwrap_or(false);
    let local_ahead = if is_local_ahead { local_n.saturating_sub(remote_n) } else { 0 };
    let remote_ahead = if is_remote_ahead { remote_n.saturating_sub(local_n) } else { 0 };

    let files = events_with_tag(events, "repositoryStatusFile").into_iter().map(|d| {
        let path = d.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        ChangedFileDto {
            is_binary: is_binary_path(&path),
            action: d.get("action").map(map_action).unwrap_or_else(|| "modify".into()),
            size: d.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
            path,
        }
    }).collect();

    StatusResultDto { branch, local_ahead, remote_ahead, files }
}

/// `flag*`/`is*` fields serialize as JSON booleans (via `u8_as_bool`); accept a
/// number too for safety.
fn json_truthy(v: &serde_json::Value) -> bool {
    v.as_bool().unwrap_or_else(|| v.as_i64().map(|n| n != 0).unwrap_or(false))
}

#[tauri::command]
pub fn lore_status(repo_path: String) -> Result<StatusResultDto, String> {
    let events = run_lore(&["status", "--repository", &repo_path])?;
    Ok(status_from(&events))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn detects_valid_identity() {
        let events = parse_events(include_str!("../tests/fixtures/auth_list.ndjson")).unwrap();
        // The captured identity's `expires` is in the past relative to a far-future clock,
        // and in the future relative to a zero clock.
        assert!(is_authenticated_from(&events, 0));
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_status_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/status.ndjson")).unwrap();
        let status = status_from(&events);
        assert!(!status.branch.is_empty());
        // Files may be empty for a clean clone; the parse must still succeed with a branch.
    }
}
