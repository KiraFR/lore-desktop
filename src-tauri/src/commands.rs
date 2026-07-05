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

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitDto {
    pub id: String,
    pub rev: u64,
    pub message: String,
    pub author: String,
    pub when: String,
    pub adds: u64,
    pub mods: u64,
    pub dels: u64,
    pub lane: u64,           // Slice A: 0 for all (linear); real lane layout is a follow-up
    pub parents: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    pub files: Vec<ChangedFileDto>,
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub commits: Vec<CommitDto>,
    pub next_cursor: Option<String>,
}

fn zero_hash(h: &str) -> bool {
    h.is_empty() || h.chars().all(|c| c == '0')
}

/// A `metadata` value is `{"tagName":"string|numeric|context","data":<v>}`.
fn metadata_value_string(value: &serde_json::Value) -> String {
    match value.get("data") {
        Some(v) if v.is_string() => v.as_str().unwrap().to_string(),
        Some(v) => v.to_string(),
        None => String::new(),
    }
}
fn metadata_value_u64(value: &serde_json::Value) -> u64 {
    value.get("data").and_then(|v| v.as_u64()).unwrap_or(0)
}

/// Format an epoch-ms timestamp as a short relative string (e.g. "2 min ago").
fn relative_time(ms: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(ms);
    let secs = now.saturating_sub(ms) / 1000;
    if secs < 60 { "just now".to_string() }
    else if secs < 3600 { format!("{} min ago", secs / 60) }
    else if secs < 86_400 { format!("{} hours ago", secs / 3600) }
    else { format!("{} days ago", secs / 86_400) }
}

/// Walk the stream: each `revisionHistoryEntry` starts a commit; the following
/// `metadata` events (until the next entry) fill message / author-id / timestamp.
/// Author ids (`created-by`) are resolved to display names via the trailing
/// `authUserInfo` events. `head` is left `None` in Slice A (history exposes a
/// branch id, not a name — real head labels are a follow-up).
fn history_from(events: &[LoreEvent]) -> HistoryPage {
    let mut users: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for d in events_with_tag(events, "authUserInfo") {
        if let (Some(id), Some(name)) =
            (d.get("id").and_then(|v| v.as_str()), d.get("name").and_then(|v| v.as_str()))
        {
            users.insert(id.to_string(), name.to_string());
        }
    }

    let mut commits: Vec<CommitDto> = Vec::new();
    let mut author_ids: Vec<String> = Vec::new();
    let mut when_ms: Vec<u64> = Vec::new();
    for ev in events {
        match ev.tag_name.as_str() {
            "revisionHistoryEntry" => {
                let d = &ev.data;
                let id = d.get("revision").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let rev = d.get("revisionNumber").and_then(|v| v.as_u64()).unwrap_or(0);
                let parents = d.get("parent").and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|p| p.as_str())
                        .filter(|p| !zero_hash(p)).map(String::from).collect())
                    .unwrap_or_default();
                commits.push(CommitDto {
                    id, rev, parents, head: None,
                    message: String::new(), author: String::new(), when: String::new(),
                    adds: 0, mods: 0, dels: 0, lane: 0, files: Vec::new(),
                });
                author_ids.push(String::new());
                when_ms.push(0);
            }
            "metadata" => {
                if let Some(i) = commits.len().checked_sub(1) {
                    let key = ev.data.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(value) = ev.data.get("value") {
                        match key {
                            "message" => commits[i].message = metadata_value_string(value),
                            "created-by" => author_ids[i] = metadata_value_string(value),
                            "timestamp" => when_ms[i] = metadata_value_u64(value),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (i, c) in commits.iter_mut().enumerate() {
        c.author = users.get(&author_ids[i]).cloned().unwrap_or_else(|| author_ids[i].clone());
        c.when = relative_time(when_ms[i]);
    }

    let next_cursor = commits.last().map(|c| c.id.clone());
    HistoryPage { commits, next_cursor }
}

#[tauri::command]
pub fn lore_history(repo_path: String, length: u32, cursor: Option<String>) -> Result<HistoryPage, String> {
    let len = length.to_string();
    let mut args: Vec<&str> = vec!["history", &len, "--repository", &repo_path];
    if let Some(ref c) = cursor {
        args.push("--revision");
        args.push(c);
    }
    let events = run_lore(&args)?;
    let mut page = history_from(&events);
    // When paging, `--revision <cursor>` re-includes the cursor commit as the
    // first entry; drop it so pages don't overlap.
    if cursor.is_some() && !page.commits.is_empty() {
        page.commits.remove(0);
    }
    Ok(page)
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

#[cfg(test)]
mod history_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_history_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/history.ndjson")).unwrap();
        let page = history_from(&events);
        assert_eq!(page.commits.len(), 2);
        assert_eq!(page.commits[0].rev, 2);
        assert_eq!(page.commits[0].message, "Add lib.rs and update main");
        assert_eq!(page.commits[0].author, "jimmy@example.com");
        assert_eq!(page.commits[0].parents.len(), 1); // rev 2 → one real parent (rev 1)
        assert!(page.commits[1].parents.is_empty());   // rev 1 is the root
        assert!(page.next_cursor.is_some());
    }
}
