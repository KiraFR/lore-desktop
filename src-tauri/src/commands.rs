use crate::lore::{events_with_tag, run_lore, run_lore_streaming, LoreEvent};
use tauri::Emitter;

/// Run a blocking body on the async runtime's worker pool so a slow or hung
/// `lore` call never blocks the UI thread — the invoke promise just pends.
pub(crate) async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(f).await {
        Ok(r) => r,
        Err(e) => Err(format!("task failed: {e}")),
    }
}

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
pub async fn lore_is_authenticated() -> Result<bool, String> {
    blocking(move || {
        let events = run_lore(&["auth", "list"])?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Ok(is_authenticated_from(&events, now_ms))
    })
    .await
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

/// Compteurs du `repositoryStatusSummary` (voir fixtures/README.md — encodage
/// pinné au lot P4). `mods` replie modifies + moves + copies : l'UI colore les
/// glyphes R/C dans la famille « modified ».
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusSummaryDto {
    pub adds: u64,
    pub mods: u64,
    pub dels: u64,
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusResultDto {
    pub branch: String,
    pub local_ahead: u64,
    pub remote_ahead: u64,
    pub revision_number: u64,
    /// The local head's revision number. `revision_number < local_revision_number`
    /// means the working copy is time-traveled to a past revision (behind chip).
    pub local_revision_number: u64,
    pub remote_available: bool,
    pub remote_authorized: bool,
    /// A merge is waiting for conflict resolution (revisionMerged* non-zero).
    pub merge_in_progress: bool,
    /// An interrupted commit/merge left a staged state (revisionStaged non-zero).
    pub staged_pending: bool,
    /// Compteurs adds/mods/dels du wire ; absent quand le CLI ne les émet pas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<StatusSummaryDto>,
    pub files: Vec<ChangedFileDto>,
}

/// Known-binary game/DCC formats — fast path, no disk access.
const BINARY_EXTS: &[&str] = &[
    "uasset", "umap", "pak",
    "png", "tga", "dds", "exr", "hdr", "tif", "tiff", "jpg", "jpeg", "webp", "gif", "psd",
    "fbx", "obj", "abc", "gltf", "glb", "blend", "ma", "mb", "max", "ztl",
    "sbs", "sbsar", "spp",
    "wav", "ogg", "mp3", "flac", "bank",
    "anim", "zip", "bin", "dll", "exe", "so", "dylib",
];
/// Known-text formats — skips the content sniff (and its disk read).
const TEXT_EXTS: &[&str] = &[
    "txt", "md", "ini", "cfg", "json", "yaml", "yml", "toml", "xml", "csv",
    "cpp", "hpp", "h", "c", "cs", "py", "rs", "js", "ts", "svelte", "css", "html",
    "uproject", "uplugin", "usf", "ush",
];

pub(crate) fn ext_of(path: &str) -> String {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match base.rsplit_once('.') {
        Some((_, e)) => e.to_ascii_lowercase(),
        None => String::new(),
    }
}

/// NUL byte in the first 8 KiB ⇒ binary. `None` when the file can't be read.
fn sniff_nul(path: &std::path::Path) -> Option<bool> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 8192];
    let n = f.read(&mut buf).ok()?;
    Some(buf[..n].contains(&0))
}

/// Extension lists first; unknown extensions get a content sniff of the local
/// working file. Unreadable (deleted, missing) files default to text.
fn is_binary(repo_root: &std::path::Path, rel_path: &str) -> bool {
    let ext = ext_of(rel_path);
    if BINARY_EXTS.contains(&ext.as_str()) {
        return true;
    }
    if TEXT_EXTS.contains(&ext.as_str()) {
        return false;
    }
    sniff_nul(&repo_root.join(rel_path)).unwrap_or(false)
}

#[cfg(test)]
mod binary_tests {
    use super::*;

    fn tmp(name: &str, content: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("lore-desktop-bintest");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn known_extensions_skip_the_disk() {
        let root = std::path::Path::new("Z:/does/not/exist");
        assert!(is_binary(root, "Content/Char/hero.blend"));
        assert!(is_binary(root, "Audio/music.BANK"));
        assert!(!is_binary(root, "Source/Player.cpp"));
    }

    #[test]
    fn unknown_extension_sniffs_content() {
        let dir = std::env::temp_dir().join("lore-desktop-bintest");
        tmp("asset.customfmt", b"BINHEader\x00\x01\x02rest");
        tmp("notes.customtxt", b"plain old text, no nul here");
        assert!(is_binary(&dir, "asset.customfmt"));
        assert!(!is_binary(&dir, "notes.customtxt"));
    }

    #[test]
    fn missing_file_defaults_to_text() {
        assert!(!is_binary(std::path::Path::new("Z:/nope"), "gone.customfmt"));
    }
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

fn status_from(events: &[LoreEvent], repo_root: &std::path::Path) -> StatusResultDto {
    let rev = events_with_tag(events, "repositoryStatusRevision").into_iter().next();
    let branch = rev.and_then(|d| d.get("branchName")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let local_n = rev.and_then(|d| d.get("revisionLocalNumber")).and_then(|v| v.as_u64()).unwrap_or(0);
    let remote_n = rev.and_then(|d| d.get("revisionRemoteNumber")).and_then(|v| v.as_u64()).unwrap_or(0);
    let is_local_ahead = rev.and_then(|d| d.get("isLocalAhead")).map(json_truthy).unwrap_or(false);
    let is_remote_ahead = rev.and_then(|d| d.get("isRemoteAhead")).map(json_truthy).unwrap_or(false);
    let local_ahead = if is_local_ahead { local_n.saturating_sub(remote_n) } else { 0 };
    let remote_ahead = if is_remote_ahead { remote_n.saturating_sub(local_n) } else { 0 };
    let revision_number = rev.and_then(|d| d.get("revisionNumber")).and_then(|v| v.as_u64()).unwrap_or(0);
    // Missing flags (older CLI) must not fake an outage — default to online.
    let remote_available = rev.and_then(|d| d.get("remoteAvailable")).map(json_truthy).unwrap_or(true);
    let remote_authorized = rev.and_then(|d| d.get("remoteAuthorized")).map(json_truthy).unwrap_or(true);

    // Merge/staged residual state (StatusBar chip). Field names pinned against
    // tests/fixtures/status_merge.ndjson + status_staged.ndjson; absent fields
    // (older CLI, no merge) default to false.
    let staged_pending = rev
        .and_then(|d| d.get("revisionStaged"))
        .and_then(|v| v.as_str())
        .map(|h| !zero_hash(h))
        .unwrap_or(false);
    // `revisionMerged` alone is NOT enough for "a merge needs resolution": a
    // COMMITTED merge keeps it set permanently (it's the merge commit's 2nd
    // parent), so keying the chip off it lit "Merge in progress" forever after
    // any clean sync-merge. An UNRESOLVED merge is the one that also leaves a
    // staged state (revisionStaged != 0 — verified: status_merge.ndjson carries
    // both, plus flagConflictUnresolved files). Require BOTH.
    let merged_head = rev
        .and_then(|d| d.get("revisionMerged"))
        .and_then(|v| v.as_str())
        .map(|h| !zero_hash(h))
        .unwrap_or(false);
    let merge_in_progress = merged_head && staged_pending;

    // Événement absent (CLI plus ancien) => None => compteurs masqués, pas de faux zéros.
    let summary = events_with_tag(events, "repositoryStatusSummary")
        .into_iter()
        .next()
        .map(|d| {
            let n = |k: &str| d.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
            StatusSummaryDto { adds: n("adds"), mods: n("modifies") + n("moves") + n("copies"), dels: n("deletes") }
        });

    let files = events_with_tag(events, "repositoryStatusFile")
        .into_iter()
        // Directory entries ("type": "directory") are containers, not changes
        // the UI can preview/stage individually — only real files are listed.
        .filter(|d| d.get("type").and_then(|v| v.as_str()) != Some("directory"))
        .map(|d| {
            let path = d.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            ChangedFileDto {
                is_binary: is_binary(repo_root, &path),
                action: d.get("action").map(map_action).unwrap_or_else(|| "modify".into()),
                size: d.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                path,
            }
        })
        .collect();

    StatusResultDto { branch, local_ahead, remote_ahead, revision_number, local_revision_number: local_n, remote_available, remote_authorized, merge_in_progress, staged_pending, summary, files }
}

/// `flag*`/`is*` fields serialize as JSON booleans (via `u8_as_bool`); accept a
/// number too for safety.
fn json_truthy(v: &serde_json::Value) -> bool {
    v.as_bool().unwrap_or_else(|| v.as_i64().map(|n| n != 0).unwrap_or(false))
}

#[tauri::command]
pub async fn lore_status(repo_path: String) -> Result<StatusResultDto, String> {
    // `--scan` reconciles the working tree so edits/adds/deletes show up in the
    // Changes view (a read-only status without it misses unstaged working changes,
    // which would leave the Commit button disabled). Non-destructive: it refreshes
    // dirty flags on the local working copy, it does not touch file contents.
    blocking(move || {
        let events = run_lore(&["status", "--scan", "--repository", &repo_path])?;
        Ok(status_from(&events, std::path::Path::new(&repo_path)))
    })
    .await
}

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

#[derive(Serialize, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInfoDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch_name: Option<String>,
    /// Repo creation time, epoch SECONDS as the CLI emits it (UI multiplies by 1000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
}

/// First `repositoryData` event (tag pinned by the Task 16 capture).
/// Absent fields stay None — the matching About row is hidden. An empty-string
/// description is treated as absent (no blank row).
fn repository_info_from(events: &[LoreEvent]) -> RepositoryInfoDto {
    let d = events_with_tag(events, "repositoryData").into_iter().next();
    let s = |d: Option<&serde_json::Value>, k: &str| {
        d.and_then(|d| d.get(k))
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .map(String::from)
    };
    RepositoryInfoDto {
        id: s(d, "id"),
        name: s(d, "name"),
        remote_url: s(d, "remoteUrl"),
        description: s(d, "description"),
        default_branch_name: s(d, "defaultBranchName"),
        created: d.and_then(|d| d.get("created").and_then(|v| v.as_u64())),
    }
}

/// Repository metadata for the read-only "About repository" panel.
#[tauri::command]
pub async fn lore_repository_info(repo_path: String) -> Result<RepositoryInfoDto, String> {
    blocking(move || {
        let events = run_lore(&["repository", "info", "--repository", &repo_path])?;
        Ok(repository_info_from(&events))
    })
    .await
}

#[cfg(test)]
mod repository_info_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_repository_data_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/repo_info.ndjson")).unwrap();
        let info = repository_info_from(&events);
        assert_eq!(info.id.as_deref(), Some("019f333af5e073d28bb117ad1596784a"));
        assert_eq!(info.name.as_deref(), Some("desktoptest1"));
        assert_eq!(info.default_branch_name.as_deref(), Some("main"));
        assert_eq!(info.created, Some(1783270930));
        // Empty-string description is normalized to None (no blank row).
        assert_eq!(info.description, None);
    }

    #[test]
    fn empty_stream_yields_all_none() {
        let events = parse_events(r#"{"tagName":"complete","data":{"status":0}}"#).unwrap();
        assert_eq!(repository_info_from(&events), RepositoryInfoDto::default());
    }
}

#[tauri::command]
pub async fn lore_repositories(server_url: String) -> Result<Vec<RepoEntryDto>, String> {
    blocking(move || {
        let events = run_lore(&["repository", "list", &server_url])?;
        Ok(repositories_from(&events))
    })
    .await
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitDto {
    pub id: String,
    pub rev: u64,
    pub message: String,
    pub author: String,
    pub when: String,
    /// Absolute epoch-ms, for the UI's exact-date tooltip.
    pub when_ms: u64,
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
    const DAY: u64 = 86_400;
    if secs < 60 { "just now".to_string() }
    else if secs < 3600 { format!("{} min ago", secs / 60) }
    else if secs < DAY { format!("{} hours ago", secs / 3600) }
    else if secs < 30 * DAY { format!("{} days ago", secs / DAY) }
    else if secs < 365 * DAY { format!("{} months ago", secs / (30 * DAY)) }
    else { format!("{} years ago", secs / (365 * DAY)) }
}

#[cfg(test)]
mod relative_time_tests {
    use super::relative_time;

    /// Epoch-ms `secs` seconds before now (the function reads the clock).
    fn ms_ago(secs: u64) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now - secs * 1000
    }

    #[test]
    fn under_a_minute_is_just_now() {
        assert_eq!(relative_time(ms_ago(5)), "just now");
        assert_eq!(relative_time(ms_ago(59)), "just now");
    }

    #[test]
    fn minutes_under_an_hour() {
        assert_eq!(relative_time(ms_ago(60)), "1 min ago");
        assert_eq!(relative_time(ms_ago(59 * 60)), "59 min ago");
    }

    #[test]
    fn hours_under_a_day() {
        assert_eq!(relative_time(ms_ago(3600)), "1 hours ago");
        assert_eq!(relative_time(ms_ago(23 * 3600)), "23 hours ago");
    }

    #[test]
    fn days_under_thirty() {
        assert_eq!(relative_time(ms_ago(86_400)), "1 days ago");
        assert_eq!(relative_time(ms_ago(29 * 86_400)), "29 days ago");
    }

    #[test]
    fn months_under_a_year() {
        assert_eq!(relative_time(ms_ago(30 * 86_400)), "1 months ago");
        assert_eq!(relative_time(ms_ago(360 * 86_400)), "12 months ago");
    }

    #[test]
    fn years_beyond() {
        assert_eq!(relative_time(ms_ago(365 * 86_400)), "1 years ago");
        assert_eq!(relative_time(ms_ago(2 * 365 * 86_400)), "2 years ago");
    }

    #[test]
    fn future_timestamp_clamps_to_just_now() {
        assert_eq!(relative_time(ms_ago(0) + 60_000), "just now");
    }
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
    let mut branch_ids: Vec<String> = Vec::new();
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
                    when_ms: 0, lane: 0, files: Vec::new(),
                });
                author_ids.push(String::new());
                when_ms.push(0);
                branch_ids.push(String::new());
            }
            "metadata" => {
                if let Some(i) = commits.len().checked_sub(1) {
                    let key = ev.data.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(value) = ev.data.get("value") {
                        match key {
                            "message" => commits[i].message = metadata_value_string(value),
                            "created-by" => author_ids[i] = metadata_value_string(value),
                            "timestamp" => when_ms[i] = metadata_value_u64(value),
                            "branch" => branch_ids[i] = metadata_value_string(value),
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
        c.when_ms = when_ms[i];
    }
    assign_lanes_by_branch(&mut commits, &branch_ids);

    let next_cursor = commits.last().map(|c| c.id.clone());
    HistoryPage { commits, next_cursor }
}

/// Assign a graph lane (column) to each commit **by its branch**, so a branch's
/// commits sit on their own lane and visibly fork from their base — even when the
/// history is topologically linear (Lore branches are linear stacks). Commits are
/// in display order (newest first); we scan oldest→newest so the trunk (the base
/// branch of the oldest commit) takes lane 0 and each stacked/merged branch that
/// appears gets the next lane. The History SVG renders the fork/rejoin edges from
/// each commit's `lane` and its parents. Lanes are per page (not continuous across
/// a pagination boundary).
fn assign_lanes_by_branch(commits: &mut [CommitDto], branch_ids: &[String]) {
    let mut lane_of: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut next: u64 = 0;
    for i in (0..commits.len()).rev() {
        let branch = branch_ids.get(i).cloned().unwrap_or_default();
        let lane = match lane_of.get(&branch) {
            Some(&l) => l,
            None => {
                let l = next;
                next += 1;
                lane_of.insert(branch, l);
                l
            }
        };
        commits[i].lane = lane;
    }
}

/// End-of-history rule: a page shorter than requested means nothing older
/// remains, so pagination must stop (`None`) instead of re-serving the tail.
fn next_cursor_for(raw_len: usize, requested: u32, last_id: Option<String>) -> Option<String> {
    if (raw_len as u32) < requested { None } else { last_id }
}

#[cfg(test)]
mod cursor_tests {
    use super::next_cursor_for;

    #[test]
    fn short_page_ends_pagination() {
        assert_eq!(next_cursor_for(3, 200, Some("x".into())), None);
    }

    #[test]
    fn full_page_keeps_cursor() {
        assert_eq!(next_cursor_for(200, 200, Some("x".into())), Some("x".into()));
    }
}

#[tauri::command]
pub async fn lore_history(repo_path: String, length: u32, cursor: Option<String>) -> Result<HistoryPage, String> {
    blocking(move || {
        let len = length.to_string();
        let mut args: Vec<&str> = vec!["history", &len, "--repository", &repo_path];
        if let Some(ref c) = cursor {
            args.push("--revision");
            args.push(c);
        }
        let events = run_lore(&args)?;
        let mut page = history_from(&events);
        let raw_len = page.commits.len();
        // When paging, `--revision <cursor>` re-includes the cursor commit as the
        // first entry; drop it so pages don't overlap.
        if cursor.is_some() && !page.commits.is_empty() {
            page.commits.remove(0);
        }
        // Label commits that are a branch's tip with the branch name (e.g. "main",
        // "feature/x") so a stacked branch's commits are distinguishable from the base.
        // Best-effort: a branch-list failure just leaves the labels off.
        if let Ok(branch_events) = run_lore(&["branch", "list", "--repository", &repo_path]) {
            let tips = branch_tips_from(&branch_events);
            for c in page.commits.iter_mut() {
                if let Some(name) = tips.get(&c.id) {
                    c.head = Some(name.clone());
                }
            }
        }
        page.next_cursor = next_cursor_for(raw_len, length, page.next_cursor.take());
        Ok(page)
    })
    .await
}

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

/// Wire payload of `lore://op-progress`. The `op_id` is generated by the
/// FRONTEND and passed through the command, so the webview listener can filter
/// on it — this distinguishes simultaneous operations (e.g. a sync while a
/// clone runs) without an extra round-trip.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpProgressPayload {
    pub op_id: String,
    pub kind: &'static str, // "clone" | "sync" | "push"
    pub done: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<&'static str>, // "bytes" | "files"
}

/// Progress-bearing tags actually observed during the Task 13 capture
/// (clone/push/sync against a real server) — replaces the earlier
/// `ends_with("Progress")` heuristic now that the real names are pinned.
/// Each op emits a DIFFERENT wire shape (see `op_progress_from`); a push or
/// sync covering several revisions emits one Begin/Progress…/End burst PER
/// revision, so `done`/`total` legitimately resets to 0 multiple times within
/// a single operation — that is real progress, not a bug.
const OP_PROGRESS_TAGS: &[&str] =
    &["repositoryCloneProgress", "branchPushFragmentProgress", "revisionSyncProgress"];

/// Map a progress event to `(done, total)` in bytes. Field names differ per
/// operation (pinned against tests/fixtures/clone_progress.ndjson and the
/// Task 13 live capture of push/sync):
/// - `repositoryCloneProgress`: nested under `count` — `bytesTransferred` / `bytesTotal`.
/// - `branchPushFragmentProgress`: top-level — `bytesTransferred` / `bytesTotal`.
/// - `revisionSyncProgress`: top-level — `bytesUpdate` / `bytesUpdateTotal`.
/// A `total` of exactly 0 means "not yet discovered" (observed before the
/// discovery pass completes) and is treated as unknown, not "already done" —
/// otherwise the first tick of an op would read as 100%.
fn op_progress_from(ev: &LoreEvent) -> Option<(u64, Option<u64>)> {
    if !OP_PROGRESS_TAGS.contains(&ev.tag_name.as_str()) {
        return None;
    }
    let d = &ev.data;
    // `count` is an object only for clone (`{"count":{"bytesTransferred":…}}`);
    // for push it's a plain fragment-count number, so only descend when it's
    // actually an object — otherwise treat `count`'s presence as unrelated.
    let src = d.get("count").filter(|v| v.is_object()).unwrap_or(d);
    let done = src
        .get("bytesTransferred")
        .or_else(|| src.get("bytesUpdate"))
        .and_then(|v| v.as_u64())?;
    let total = src
        .get("bytesTotal")
        .or_else(|| src.get("bytesUpdateTotal"))
        .and_then(|v| v.as_u64())
        .filter(|&t| t > 0);
    Some((done, total))
}

/// Unit of the progress counts — confirmed bytes by the Task 13 capture: the
/// clone fixture's `count.bytesTotal` (202) equals the exact sum of the 6
/// tracked file sizes in the test repo.
const OP_PROGRESS_UNIT: Option<&'static str> = Some("bytes");

/// Minimum spacing between two `lore://op-progress` emits for the same
/// operation, so a fast-ticking child (thousands of small files) doesn't
/// flood the webview IPC channel.
const OP_PROGRESS_THROTTLE: std::time::Duration = std::time::Duration::from_millis(33);

/// Whether a progress tick should be emitted now: the very first tick and the
/// final tick (`done >= total`, once `total` is known) always go through so
/// the UI never stalls visually short of 100%; everything else is throttled
/// to at most one emit per [`OP_PROGRESS_THROTTLE`].
fn should_emit(last: Option<std::time::Instant>, done: u64, total: Option<u64>) -> bool {
    let is_final = total.is_some_and(|t| done >= t);
    let due = last.is_none_or(|t| t.elapsed() >= OP_PROGRESS_THROTTLE);
    is_final || due
}

/// Run a long lore operation on the streaming runner, relaying progress
/// events to the webview as `lore://op-progress`. Emits are throttled to
/// ~30 Hz (see [`OP_PROGRESS_THROTTLE`]) to bound IPC volume on fast-ticking
/// operations, but the final tick (`done == total`) is always emitted
/// unthrottled so the progress bar never visually stalls short of 100%.
/// Stall (60 s of silence) kills the child and surfaces the same error toast
/// as any failed operation.
fn run_lore_op(
    app: &tauri::AppHandle,
    kind: &'static str,
    op_id: &str,
    args: &[&str],
) -> Result<Vec<LoreEvent>, String> {
    let mut last_emit: Option<std::time::Instant> = None;
    let mut on_event = |ev: &LoreEvent| {
        if let Some((done, total)) = op_progress_from(ev) {
            if should_emit(last_emit, done, total) {
                last_emit = Some(std::time::Instant::now());
                let _ = app.emit(
                    "lore://op-progress",
                    OpProgressPayload { op_id: op_id.to_string(), kind, done, total, unit: OP_PROGRESS_UNIT },
                );
            }
        }
    };
    run_lore_streaming(args, &mut on_event)
}

/// `op_id` is required once the frontend sends it (Task 16); until then a
/// missing id degrades to the empty string rather than failing the op.
fn op_id_or_default(op_id: Option<String>) -> String {
    op_id.unwrap_or_default()
}

/// Clone `<server_url>/<repo_id>` into `<dest_parent>/<repo_name>` and return
/// the created path. Streaming runner: progress is relayed as
/// `lore://op-progress` (filtered by the frontend-generated `op_id`) and a
/// stalled transfer errors after 60 s of silence — a long but advancing clone
/// is never killed (the old flat 45 s cap made big clones fail).
#[tauri::command]
pub async fn lore_clone(
    app: tauri::AppHandle,
    server_url: String,
    repo_id: String,
    repo_name: String,
    dest_parent: String,
    op_id: Option<String>,
) -> Result<String, String> {
    blocking(move || {
        let op_id = op_id_or_default(op_id);
        let (url, path) = build_clone_args(&server_url, &repo_id, &repo_name, &dest_parent);
        run_lore_op(&app, "clone", &op_id, &["clone", &url, &path])?;
        Ok(path)
    })
    .await
}

#[derive(Serialize, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SharedStoreStatusDto {
    pub exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Global "use automatically" toggle, when the CLI reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_use: Option<bool>,
}

/// Parse `shared-store info --json`. Tag `sharedStoreInfo` (pinned Task 5); the
/// per-remote fields are PARALLEL ARRAYS `remoteUrls`/`paths`/`exists`, plus a
/// global `useAutomatically` (0/1). We surface the first store's path + whether
/// any store exists + the global auto-use flag.
fn shared_store_status_from(events: &[LoreEvent]) -> SharedStoreStatusDto {
    fn truthy(v: &serde_json::Value) -> bool {
        v.as_bool().unwrap_or_else(|| v.as_u64().map(|n| n != 0).unwrap_or(false))
    }
    let info = events_with_tag(events, "sharedStoreInfo").into_iter().next();
    let path = info
        .and_then(|d| d.get("paths"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().find_map(|p| p.as_str().filter(|s| !s.is_empty())))
        .map(String::from);
    let exists = path.is_some()
        || info
            .and_then(|d| d.get("exists"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(truthy))
            .unwrap_or(false);
    let auto_use = info.and_then(|d| d.get("useAutomatically")).map(truthy);
    SharedStoreStatusDto { exists, path, auto_use }
}

/// Whether a shared object store exists on this machine (and where) + the global
/// auto-use flag. A CLI error here means "no store yet" — a normal answer.
#[tauri::command]
pub async fn lore_shared_store_status() -> Result<SharedStoreStatusDto, String> {
    blocking(move || match run_lore(&["shared-store", "info"]) {
        Ok(events) => Ok(shared_store_status_from(&events)),
        Err(_) => Ok(SharedStoreStatusDto::default()),
    })
    .await
}

/// Enable the shared store for clones: create a per-remote store for `server_url`
/// if none exists yet (create ERRORS on an existing store, so we guard), then turn
/// on the global automatic-use flag.
#[tauri::command]
pub async fn lore_shared_store_enable(server_url: String) -> Result<(), String> {
    blocking(move || {
        let has_store = run_lore(&["shared-store", "info"])
            .map(|ev| shared_store_status_from(&ev).exists)
            .unwrap_or(false);
        if !has_store {
            run_lore(&["shared-store", "create", &server_url])?;
        }
        run_lore(&["shared-store", "set-use-automatically", "true"])?;
        Ok(())
    })
    .await
}

/// Turn off automatic shared-store use (the store itself is kept on disk).
#[tauri::command]
pub async fn lore_shared_store_disable() -> Result<(), String> {
    blocking(move || {
        run_lore(&["shared-store", "set-use-automatically", "false"])?;
        Ok(())
    })
    .await
}

#[cfg(test)]
mod shared_store_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_existing_store_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/shared_store_info.ndjson")).unwrap();
        let s = shared_store_status_from(&events);
        assert!(s.exists);
        assert!(s.path.as_deref().is_some_and(|p| p.contains("shared_store")), "path was {:?}", s.path);
        assert_eq!(s.auto_use, Some(false)); // useAutomatically:0 in the fixture
    }

    #[test]
    fn parses_no_store_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/shared_store_info_none.ndjson")).unwrap();
        let s = shared_store_status_from(&events);
        assert!(!s.exists);
        assert_eq!(s.path, None);
        assert_eq!(s.auto_use, Some(false)); // event present, arrays empty
    }

    #[test]
    fn no_info_event_yields_default() {
        let events = parse_events(r#"{"tagName":"complete","data":{"status":0}}"#).unwrap();
        assert_eq!(shared_store_status_from(&events), SharedStoreStatusDto::default());
    }
}

#[tauri::command]
pub async fn lore_sign_in(server_url: String, auth_url: Option<String>) -> Result<(), String> {
    blocking(move || {
        let mut cmd = std::process::Command::new("lore");
        cmd.arg("login");
        if let Some(ref a) = auth_url {
            cmd.arg("--auth-url").arg(a);
        }
        cmd.arg(&server_url);
        let status = cmd.status().map_err(|e| format!("failed to launch lore login: {e}"))?;
        if status.success() { Ok(()) } else { Err("sign-in failed or was cancelled".to_string()) }
    })
    .await
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdentityDto {
    pub id: String,
    pub email: String,
}

/// First `authUserInfo { id, name }` event — `name` is the account email.
fn identity_from(events: &[LoreEvent]) -> Option<IdentityDto> {
    events_with_tag(events, "authUserInfo").into_iter().find_map(|d| {
        let id = d.get("id").and_then(|v| v.as_str())?;
        let email = d.get("name").and_then(|v| v.as_str())?;
        Some(IdentityDto { id: id.to_string(), email: email.to_string() })
    })
}

/// The signed-in identity as the repo's server knows it. `lore auth info`
/// only answers inside a working copy (it needs the repo's auth endpoint),
/// so there is no identity to show until a repository is open.
#[tauri::command]
pub async fn lore_identity(repo_path: String) -> Result<IdentityDto, String> {
    blocking(move || {
        let events = run_lore(&["auth", "info", "--repository", &repo_path])?;
        identity_from(&events).ok_or_else(|| "no identity".to_string())
    })
    .await
}

#[tauri::command]
pub async fn lore_sign_out() -> Result<(), String> {
    blocking(move || {
        run_lore(&["auth", "logout"])?;
        Ok(())
    })
    .await
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_auth_user_info() {
        let sample = concat!(
            r#"{"tagName":"authUserInfo","data":{"id":"8c25b13e","name":"jimmy@studio.dev"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let id = identity_from(&events).unwrap();
        assert_eq!(id.id, "8c25b13e");
        assert_eq!(id.email, "jimmy@studio.dev");
    }

    #[test]
    fn no_event_is_none() {
        let events = parse_events(r#"{"tagName":"complete","data":{"status":0}}"#).unwrap();
        assert!(identity_from(&events).is_none());
    }
}

/// `lore lock` subcommand for a lock/unlock toggle.
fn lock_subcommand(lock: bool) -> &'static str {
    if lock {
        "acquire"
    } else {
        "release"
    }
}

fn require_commit_message(message: &str) -> Result<(), String> {
    if message.trim().is_empty() {
        Err("commit message is required".to_string())
    } else {
        Ok(())
    }
}

/// Stage the working tree then commit the selected files. `exclude` (unchecked
/// files) are unstaged before the commit so only the checked ones are recorded.
#[tauri::command]
pub async fn lore_commit(repo_path: String, message: String, exclude: Vec<String>) -> Result<(), String> {
    require_commit_message(&message)?;
    blocking(move || {
        // Stage everything (incl. deletions), then drop the unchecked files so the
        // commit records only the selected ones; the rest stay pending. `unstage`
        // resolves a relative path against the process cwd, so pass an absolute path
        // (same gotcha as lock/diff/resolve).
        run_lore(&["stage", ".", "--scan", "--repository", &repo_path])?;
        for path in &exclude {
            let abs = std::path::Path::new(&repo_path).join(path);
            let abs_str = abs.to_string_lossy();
            run_lore(&["unstage", &abs_str, "--repository", &repo_path])?;
        }
        run_lore(&["commit", &message, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// Rewrite the last local commit's message via `lore revision amend <MESSAGE>`.
/// Only valid for an unpushed commit — the frontend gates on `localAhead > 0`.
#[tauri::command]
pub async fn lore_amend(repo_path: String, message: String) -> Result<(), String> {
    require_commit_message(&message)?;
    blocking(move || {
        run_lore(&["revision", "amend", &message, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn lore_push(app: tauri::AppHandle, repo_path: String, op_id: Option<String>) -> Result<(), String> {
    blocking(move || {
        let op_id = op_id_or_default(op_id);
        run_lore_op(&app, "push", &op_id, &["push", "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// Discard a file's working changes, restoring the committed version. `lore reset`
/// resolves a relative path against the process cwd, so pass an absolute path.
#[tauri::command]
pub async fn lore_discard_file(repo_path: String, path: String) -> Result<(), String> {
    blocking(move || {
        let abs = std::path::Path::new(&repo_path).join(&path);
        let abs_str = abs.to_string_lossy();
        run_lore(&["reset", &abs_str, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// A short-lived, unique scratch dir for backing up files during an undo.
fn undo_backup_dir() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("lore-undo-{}-{}", std::process::id(), nanos))
}

/// Undo the last local commit and return its changes to the pending set.
///
/// Lore's `branch reset` is HARD (it syncs the working tree to the target), so it
/// alone would discard the commit's changes. To keep them, we: (1) list the commit's
/// files (diff `parent` → working tree, which still holds the committed content),
/// (2) back up the added/modified files, (3) reset the branch to `parent`, then
/// (4) re-apply — restore added/modified files, re-delete files the commit deleted.
/// The result: the branch tip moves back and the changes reappear as pending.
/// Caller must ensure the working tree is otherwise clean (== the tip).
#[tauri::command]
pub async fn lore_undo_commit(repo_path: String, parent_revision: String) -> Result<(), String> {
    blocking(move || undo_commit_blocking(&repo_path, &parent_revision)).await
}

fn undo_commit_blocking(repo_path: &str, parent_revision: &str) -> Result<(), String> {
    use std::fs;
    let repo = std::path::Path::new(repo_path);

    // 1. The commit's files (working tree still == the tip here).
    let diff_events = run_lore(&["diff", "--source", parent_revision, "--repository", repo_path])?;
    let files = commit_files_from(&diff_events);

    // 2. Back up the content of added/modified files (their on-disk = committed version).
    let backup = undo_backup_dir();
    for f in &files {
        if f.action == "delete" {
            continue; // nothing to back up — the file is already gone on disk
        }
        let src = repo.join(&f.path);
        if !src.exists() {
            continue;
        }
        let dst = backup.join(&f.path);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("backup mkdir failed: {e}"))?;
        }
        fs::copy(&src, &dst).map_err(|e| format!("backup copy failed for {}: {e}", f.path))?;
    }

    // 3. Hard reset the branch to the parent (working tree reverts to parent).
    if let Err(e) = run_lore(&["branch", "reset", parent_revision, "--repository", repo_path]) {
        let _ = fs::remove_dir_all(&backup); // nothing mutated on disk yet
        return Err(e);
    }

    // 4. Re-apply the changes as pending: restore added/modified, re-delete deleted.
    for f in &files {
        let target = repo.join(&f.path);
        if f.action == "delete" {
            let _ = fs::remove_file(&target);
            continue;
        }
        let src = backup.join(&f.path);
        if !src.exists() {
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("restore mkdir failed: {e}"))?;
        }
        // On restore failure, keep the backup dir so nothing is lost.
        fs::copy(&src, &target).map_err(|e| format!("restore failed for {} (backup kept at {}): {e}", f.path, backup.display()))?;
    }

    let _ = fs::remove_dir_all(&backup);
    Ok(())
}


/// Plain `lore sync` — pulls/merges the remote into the local branch
/// non-destructively (NO `--reset`, which would discard local modifications).
#[tauri::command]
pub async fn lore_sync(app: tauri::AppHandle, repo_path: String, op_id: Option<String>) -> Result<(), String> {
    blocking(move || {
        let op_id = op_id_or_default(op_id);
        run_lore_op(&app, "sync", &op_id, &["sync", "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// Time travel: sync the working copy to a specific revision hash (streaming,
/// same progress relay + stall detection as a plain sync). The CLI does not
/// refuse a dirty tree (verified) — the UI gates it on a clean tree instead.
#[tauri::command]
pub async fn lore_sync_to(app: tauri::AppHandle, repo_path: String, revision: String, op_id: Option<String>) -> Result<(), String> {
    blocking(move || {
        let op_id = op_id_or_default(op_id);
        run_lore_op(&app, "sync", &op_id, &["sync", &revision, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn lore_set_lock(repo_path: String, path: String, lock: bool) -> Result<(), String> {
    blocking(move || {
        // `lore lock` resolves a relative path against the process cwd, not
        // `--repository`, so build an absolute path inside the repo.
        let abs = std::path::Path::new(&repo_path).join(&path);
        let abs_str = abs.to_string_lossy();
        run_lore(&["lock", lock_subcommand(lock), &abs_str, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LockEntryDto {
    pub path: String,
    pub holder: String,
    pub when: String,
}

/// The signed-in user's id (from `auth list`), best-effort — used to label their
/// own locks as "you". Empty if it can't be determined.
fn current_user_id() -> String {
    run_lore(&["auth", "list"])
        .ok()
        .and_then(|evs| {
            events_with_tag(&evs, "authIdentity")
                .into_iter()
                .find_map(|d| d.get("userId").and_then(|v| v.as_str()).map(String::from))
        })
        .unwrap_or_default()
}

/// Map `lockFileQuery` events → `LockEntry`, resolving each owner id to a display
/// name via the trailing `authUserInfo` events (own locks show as "you").
fn locks_from(events: &[LoreEvent], me: &str) -> Vec<LockEntryDto> {
    let mut users: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for d in events_with_tag(events, "authUserInfo") {
        if let (Some(id), Some(name)) =
            (d.get("id").and_then(|v| v.as_str()), d.get("name").and_then(|v| v.as_str()))
        {
            users.insert(id.to_string(), name.to_string());
        }
    }
    events_with_tag(events, "lockFileQuery")
        .into_iter()
        .map(|d| {
            let owner = d.get("owner").and_then(|v| v.as_str()).unwrap_or("");
            let holder = if !me.is_empty() && owner == me {
                "you".to_string()
            } else {
                users.get(owner).cloned().unwrap_or_else(|| owner.to_string())
            };
            LockEntryDto {
                path: d.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                holder,
                when: relative_time(d.get("lockedAt").and_then(|v| v.as_u64()).unwrap_or(0)),
            }
        })
        .collect()
}

#[tauri::command]
pub async fn lore_locks(repo_path: String) -> Result<Vec<LockEntryDto>, String> {
    blocking(move || {
        let me = current_user_id();
        let events = run_lore(&["lock", "query", "--repository", &repo_path])?;
        Ok(locks_from(&events, &me))
    })
    .await
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BranchDto {
    pub name: String,
    pub current: bool,
    /// "local" (existe dans la working copy) ou "remote" (remote-only après
    /// dédup). Champ wire absent (CLI plus ancien) => "local" : défaut sûr,
    /// pas de section Remote fantôme.
    pub location: String,
}

/// Union of `branchListEntry` events (which stream once per location, local then
/// remote) deduped by name. `current` folds `isCurrent` across every entry for a
/// name (only local entries carry it); a name with ANY local entry is "local",
/// otherwise "remote" (remote-only). Archived branches are dropped. First-seen
/// order is preserved, so local branches come first and remote-only ones append.
fn branches_from(events: &[LoreEvent]) -> Vec<BranchDto> {
    let mut order: Vec<String> = Vec::new();
    let mut current: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut local: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for d in events_with_tag(events, "branchListEntry") {
        if d.get("archived").map(json_truthy).unwrap_or(false) {
            continue;
        }
        let name = match d.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };
        if d.get("location").and_then(|v| v.as_str()).unwrap_or("local") == "local" {
            local.insert(name.clone());
        }
        if d.get("isCurrent").map(json_truthy).unwrap_or(false) {
            current.insert(name.clone());
        }
        if seen.insert(name.clone()) {
            order.push(name);
        }
    }
    order
        .into_iter()
        .map(|name| BranchDto {
            current: current.contains(&name),
            location: if local.contains(&name) { "local" } else { "remote" }.to_string(),
            name,
        })
        .collect()
}

#[tauri::command]
pub async fn lore_branches(repo_path: String) -> Result<Vec<BranchDto>, String> {
    blocking(move || {
        let events = run_lore(&["branch", "list", "--repository", &repo_path])?;
        Ok(branches_from(&events))
    })
    .await
}

/// Map each branch's local tip revision hash → branch name, from `branchListEntry`
/// events. Only local tips are used since they align with the local revision
/// history; archived branches are skipped. Used to label branch-head commits.
fn branch_tips_from(events: &[LoreEvent]) -> std::collections::HashMap<String, String> {
    let mut tips = std::collections::HashMap::new();
    for d in events_with_tag(events, "branchListEntry") {
        if d.get("location").and_then(|v| v.as_str()) != Some("local") {
            continue;
        }
        if d.get("archived").map(json_truthy).unwrap_or(false) {
            continue;
        }
        if let (Some(latest), Some(name)) = (
            d.get("latest").and_then(|v| v.as_str()),
            d.get("name").and_then(|v| v.as_str()),
        ) {
            if !zero_hash(latest) {
                tips.insert(latest.to_string(), name.to_string());
            }
        }
    }
    tips
}

#[tauri::command]
pub async fn lore_switch_branch(repo_path: String, name: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&["branch", "switch", &name, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// `lore branch create` makes the branch from the current latest and auto-switches
/// to it, so no separate switch is needed. The base is always the current HEAD.
#[tauri::command]
pub async fn lore_create_branch(repo_path: String, name: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&["branch", "create", &name, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// Archive a branch: it disappears from `branch list` output (and thus from the
/// UI, which already filters `archived: true`); nothing is deleted.
#[tauri::command]
pub async fn lore_archive_branch(repo_path: String, name: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&["branch", "archive", &name, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// The set of file paths changed in a revision-range diff (`fileDiff` events).
fn pushed_paths_from(events: &[LoreEvent]) -> std::collections::HashSet<String> {
    events_with_tag(events, "fileDiff")
        .into_iter()
        .filter_map(|d| d.get("path").and_then(|v| v.as_str()).map(String::from))
        .collect()
}

/// True for a revision hash that is unset/all-zero (no remote tip yet → first push).
fn is_zero_revision(rev: &str) -> bool {
    rev.is_empty() || rev.chars().all(|c| c == '0')
}

/// Files the signed-in user holds locked AND that are part of the pending push
/// (the diff between the remote tip and the local tip). Used to offer releasing
/// just-pushed locks after a push. Empty when nothing is ahead; on a first push
/// (no remote tip yet) every held lock qualifies since the whole tree is pushed.
#[tauri::command]
pub async fn lore_pushed_lock_files(repo_path: String) -> Result<Vec<String>, String> {
    blocking(move || {
        // 1. My held locks; nothing to offer if I hold none.
        let me = current_user_id();
        let lock_events = run_lore(&["lock", "query", "--repository", &repo_path])?;
        let mine: Vec<String> = locks_from(&lock_events, &me)
            .into_iter()
            .filter(|l| l.holder == "you")
            .map(|l| l.path)
            .collect();
        if mine.is_empty() {
            return Ok(vec![]);
        }

        // 2. Remote vs local tip; nothing to push if they match.
        let status_events = run_lore(&["status", "--repository", &repo_path])?;
        let rev = events_with_tag(&status_events, "repositoryStatusRevision")
            .into_iter()
            .next();
        let (remote, local) = match rev {
            Some(d) => (
                d.get("revisionRemote").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                d.get("revisionLocal").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            ),
            None => return Ok(vec![]),
        };
        if is_zero_revision(&local) || remote == local {
            return Ok(vec![]);
        }

        // 3. First push: the whole working tree is pushed, so every held lock qualifies.
        if is_zero_revision(&remote) {
            return Ok(mine);
        }

        // 4. Otherwise intersect my locks with the pushed changeset.
        let diff_events = run_lore(&[
            "diff", "--source", &remote, "--target", &local, "--repository", &repo_path,
        ])?;
        let pushed = pushed_paths_from(&diff_events);
        Ok(mine.into_iter().filter(|p| pushed.contains(p)).collect())
    })
    .await
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitFileDto {
    pub path: String,
    pub action: String, // "add" | "modify" | "delete" | "move" | "copy"
}

/// Map `fileDiff` events (`{ path, action }`) onto the UI's per-commit file list.
fn commit_files_from(events: &[LoreEvent]) -> Vec<CommitFileDto> {
    events_with_tag(events, "fileDiff")
        .into_iter()
        .map(|d| CommitFileDto {
            path: d.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            action: d.get("action").map(map_action).unwrap_or_else(|| "modify".into()),
        })
        .collect()
}

/// The files a single commit changed = the diff between the commit and its first
/// parent. Fetched lazily when a commit is selected in History (one diff per
/// click, never eagerly for every row). A root commit (no parent) has no diff
/// base, so it returns an empty list.
#[tauri::command]
pub async fn lore_commit_files(
    repo_path: String,
    revision: String,
    parent: String,
) -> Result<Vec<CommitFileDto>, String> {
    if is_zero_revision(&parent) {
        return Ok(vec![]);
    }
    blocking(move || {
        let events = run_lore(&[
            "diff", "--source", &parent, "--target", &revision, "--repository", &repo_path,
        ])?;
        Ok(commit_files_from(&events))
    })
    .await
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileRevisionDto {
    pub revision: String,
    pub revision_number: u64,
    pub action: String,
    pub size: u64,
    pub message: String,
    pub author: String,
    pub when: String,
    pub when_ms: u64,
}

/// Walk the stream: each `fileHistory` starts a revision; the following
/// `metadata` events fill message / author-id / timestamp. `author` holds the
/// raw user id here — the command resolves ids to names afterwards.
fn file_history_from(events: &[LoreEvent]) -> Vec<FileRevisionDto> {
    let mut out: Vec<FileRevisionDto> = Vec::new();
    for ev in events {
        match ev.tag_name.as_str() {
            "fileHistory" => {
                let d = &ev.data;
                out.push(FileRevisionDto {
                    revision: d.get("revision").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    revision_number: d.get("revisionNumber").and_then(|v| v.as_u64()).unwrap_or(0),
                    action: d.get("action").map(map_action).unwrap_or_else(|| "modify".into()),
                    size: d.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                    message: String::new(),
                    author: String::new(),
                    when: String::new(),
                    when_ms: 0,
                });
            }
            "metadata" => {
                if let Some(last) = out.last_mut() {
                    let key = ev.data.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(value) = ev.data.get("value") {
                        match key {
                            "message" => last.message = metadata_value_string(value),
                            "created-by" => last.author = metadata_value_string(value),
                            "timestamp" => last.when_ms = metadata_value_u64(value),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for r in out.iter_mut() {
        r.when = relative_time(r.when_ms);
    }
    out
}

/// Resolve the distinct author ids to display names in ONE `auth info` call.
/// Best-effort: on failure the UUIDs stay (the UI truncates them).
fn resolve_authors(repo_path: &str, revs: &mut [FileRevisionDto]) {
    let mut ids: Vec<String> = revs.iter().map(|r| r.author.clone()).filter(|s| !s.is_empty()).collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return;
    }
    let mut args: Vec<&str> = vec!["auth", "info"];
    args.extend(ids.iter().map(|s| s.as_str()));
    args.extend(["--repository", repo_path]);
    let Ok(events) = run_lore(&args) else { return };
    let mut users = std::collections::HashMap::new();
    for d in events_with_tag(&events, "authUserInfo") {
        if let (Some(id), Some(name)) = (d.get("id").and_then(|v| v.as_str()), d.get("name").and_then(|v| v.as_str())) {
            users.insert(id.to_string(), name.to_string());
        }
    }
    for r in revs.iter_mut() {
        if let Some(name) = users.get(&r.author) {
            r.author = name.clone();
        }
    }
}

/// Revision timeline of one file. Fetched lazily when a file is selected.
#[tauri::command]
pub async fn lore_file_history(repo_path: String, path: String) -> Result<Vec<FileRevisionDto>, String> {
    blocking(move || {
        // `lore file` resolves relative paths against the process cwd (same
        // gotcha as lock/diff/reset) — pass an absolute path.
        let abs = std::path::Path::new(&repo_path).join(&path);
        let abs_str = abs.to_string_lossy();
        let events = run_lore(&["file", "history", &abs_str, "--repository", &repo_path])?;
        let mut revs = file_history_from(&events);
        resolve_authors(&repo_path, &mut revs);
        Ok(revs)
    })
    .await
}

#[cfg(test)]
mod file_history_tests {
    use super::*;
    use crate::lore::parse_events;

    const STREAM: &str = concat!(
        r#"{"tagName":"fileHistory","data":{"path":"a.png","revision":"r3","revisionNumber":3,"size":42,"action":"keep"}}"#, "\n",
        r#"{"tagName":"metadata","data":{"key":"timestamp","value":{"tagName":"numeric","data":1783331197445}}}"#, "\n",
        r#"{"tagName":"metadata","data":{"key":"message","value":{"tagName":"string","data":"tweak"}}}"#, "\n",
        r#"{"tagName":"metadata","data":{"key":"created-by","value":{"tagName":"string","data":"u1"}}}"#, "\n",
        r#"{"tagName":"fileHistory","data":{"path":"a.png","revision":"r1","revisionNumber":1,"size":51,"action":"add"}}"#, "\n",
        r#"{"tagName":"metadata","data":{"key":"message","value":{"tagName":"string","data":"import"}}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn parses_revisions_with_metadata() {
        let events = parse_events(STREAM).unwrap();
        let revs = file_history_from(&events);
        assert_eq!(revs.len(), 2);
        assert_eq!(revs[0].revision_number, 3);
        assert_eq!(revs[0].action, "modify"); // keep → modify
        assert_eq!(revs[0].message, "tweak");
        assert_eq!(revs[0].author, "u1");
        assert_eq!(revs[0].when_ms, 1783331197445);
        assert_eq!(revs[1].action, "add");
        assert_eq!(revs[1].message, "import");
    }
}

/// Sizes at the current repository revision, from `lore file info <paths…>`
/// (batch). Keyed by the path as the event reports it, `\` normalized to `/`.
/// Event/field names pinned against tests/fixtures/file_info.ndjson.
fn file_sizes_from(events: &[LoreEvent]) -> std::collections::HashMap<String, u64> {
    let mut out = std::collections::HashMap::new();
    for d in events_with_tag(events, "fileInfo") {
        if let (Some(path), Some(size)) = (
            d.get("path").and_then(|v| v.as_str()),
            d.get("size").and_then(|v| v.as_u64()),
        ) {
            out.insert(path.replace('\\', "/"), size);
        }
    }
    out
}

/// Match each requested repo-relative path against the reported sizes: exact
/// key first, else a suffix match — but ONLY when exactly one reported key
/// ends with `/{path}` (defensive: some CLI builds may echo back the
/// absolute paths they were given). Two or more candidates is ambiguous, so
/// it's treated the same as zero: absent. Unmatched paths are simply absent
/// from the result — never a fake 0.
fn relative_sizes(
    reported: &std::collections::HashMap<String, u64>,
    paths: &[String],
) -> std::collections::HashMap<String, u64> {
    let mut out = std::collections::HashMap::new();
    for rel in paths {
        let norm = rel.replace('\\', "/");
        let found = reported.get(&norm).copied().or_else(|| {
            let suffix = format!("/{norm}");
            let mut it = reported.iter().filter(|(k, _)| k.ends_with(&suffix));
            match (it.next(), it.next()) {
                (Some((_, v)), None) => Some(*v),
                _ => None,
            }
        });
        if let Some(size) = found {
            out.insert(rel.clone(), size);
        }
    }
    out
}

/// Split `items` (paired with a length-bearing key, e.g. an absolute path)
/// into consecutive chunks whose cumulative key length stays under `budget`.
/// Every chunk has at least one item — a single item longer than `budget`
/// still gets its own chunk rather than being dropped. Order is preserved
/// and nothing is lost across chunks.
fn chunk_by_arg_len<'a>(items: &'a [(String, String)], budget: usize) -> Vec<&'a [(String, String)]> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut acc = 0usize;
    for (i, (key, _)) in items.iter().enumerate() {
        let len = key.len();
        if i > start && acc + len > budget {
            chunks.push(&items[start..i]);
            start = i;
            acc = 0;
        }
        acc += len;
    }
    if start < items.len() {
        chunks.push(&items[start..]);
    }
    chunks
}

/// Cumulative absolute-path-length budget per `lore file info` batch call.
/// Windows caps a single command line around 32 767 chars; this stays well
/// under that even with the extra flags/spacing overhead.
const FILE_SIZES_ARG_BUDGET: usize = 25_000;

/// Repository-revision ("old") sizes of the given files, via `lore file info`
/// batched into chunks under [`FILE_SIZES_ARG_BUDGET`] cumulative chars of
/// absolute path (Windows caps a single command line around 32 767 chars, and
/// `lore file` resolves relative paths against the process cwd, so paths are
/// passed absolute — same gotcha as lock/diff/reset).
///
/// Callers are expected to pass only modify/delete paths from a diff. Pure
/// enrichment (the frontend calls it fire-and-forget after status): a chunk
/// whose `lore file info` call fails is simply skipped, degrading only the
/// files in that chunk. This command therefore always returns `Ok`,
/// accumulating whatever chunks succeeded — an empty map if every chunk
/// failed — and never `Err`.
#[tauri::command]
pub async fn lore_file_sizes(
    repo_path: String,
    paths: Vec<String>,
) -> Result<std::collections::HashMap<String, u64>, String> {
    if paths.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    blocking(move || {
        let abs_and_rel: Vec<(String, String)> = paths
            .iter()
            .map(|p| {
                let abs = std::path::Path::new(&repo_path).join(p).to_string_lossy().into_owned();
                (abs, p.clone())
            })
            .collect();
        let mut out = std::collections::HashMap::new();
        for chunk in chunk_by_arg_len(&abs_and_rel, FILE_SIZES_ARG_BUDGET) {
            let mut args: Vec<&str> = vec!["file", "info"];
            args.extend(chunk.iter().map(|(abs, _)| abs.as_str()));
            args.extend(["--repository", &repo_path]);
            if let Ok(events) = run_lore(&args) {
                let chunk_paths: Vec<String> = chunk.iter().map(|(_, rel)| rel.clone()).collect();
                out.extend(relative_sizes(&file_sizes_from(&events), &chunk_paths));
            }
        }
        Ok(out)
    })
    .await
}

#[cfg(test)]
mod file_sizes_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_file_info_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/file_info.ndjson")).unwrap();
        let sizes = file_sizes_from(&events);
        assert!(!sizes.is_empty(), "the captured fixture must yield at least one size");
        assert_eq!(sizes.get("README.md"), Some(&42));
        assert_eq!(sizes.get("notify-test.txt"), Some(&24));
    }

    const SAMPLE: &str = concat!(
        r#"{"tagName":"fileInfo","data":{"path":"C:/Users/jimmy/lore-test-repo/notes.txt","size":420}}"#, "\n",
        r#"{"tagName":"fileInfo","data":{"path":"C:/Users/jimmy/lore-test-repo/Content/T_Cliff.uasset","size":4093640}}"#, "\n",
        r#"{"tagName":"fileInfo","data":{"path":"C:/Users/jimmy/lore-test-repo/Content/T_Cliff2.uasset","size":123}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    // Two different repos reporting the same leaf name → suffix match is
    // ambiguous and must be dropped, not guessed.
    const AMBIGUOUS: &str = concat!(
        r#"{"tagName":"fileInfo","data":{"path":"C:/repo/a/notes.txt","size":1}}"#, "\n",
        r#"{"tagName":"fileInfo","data":{"path":"C:/repo/b/notes.txt","size":2}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn maps_reported_paths_back_to_relative() {
        let events = parse_events(SAMPLE).unwrap();
        let reported = file_sizes_from(&events);
        let out = relative_sizes(
            &reported,
            &[
                "notes.txt".to_string(),
                "Content/T_Cliff.uasset".to_string(),
                "gone.txt".to_string(),
                r"Content\T_Cliff2.uasset".to_string(),
            ],
        );
        assert_eq!(out.get("notes.txt"), Some(&420));
        assert_eq!(out.get("Content/T_Cliff.uasset"), Some(&4093640));
        // Unreported file → absent from the map, never a fake 0.
        assert!(!out.contains_key("gone.txt"));
        // Windows-style backslash request matches the `/`-reported path, and
        // the output key keeps the original backslash form (it's keyed by
        // what the caller asked for, not the normalized form).
        assert_eq!(out.get(r"Content\T_Cliff2.uasset"), Some(&123));
    }

    #[test]
    fn ambiguous_suffix_match_is_dropped() {
        let events = parse_events(AMBIGUOUS).unwrap();
        let reported = file_sizes_from(&events);
        let out = relative_sizes(&reported, &["notes.txt".to_string()]);
        // Two reported paths end with "/notes.txt" — can't tell which one the
        // caller meant, so it's absent rather than a guess.
        assert!(!out.contains_key("notes.txt"));
    }

    #[test]
    fn chunk_fits_in_one_chunk_under_budget() {
        let items: Vec<(String, String)> = vec![
            ("C:/repo/a.txt".to_string(), "a.txt".to_string()),
            ("C:/repo/b.txt".to_string(), "b.txt".to_string()),
        ];
        let chunks = chunk_by_arg_len(&items, 1000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 2);
    }

    #[test]
    fn chunk_splits_when_over_budget() {
        let items: Vec<(String, String)> = vec![
            ("a".repeat(10), "r1".to_string()),
            ("b".repeat(10), "r2".to_string()),
            ("c".repeat(10), "r3".to_string()),
        ];
        let chunks = chunk_by_arg_len(&items, 15);
        assert!(chunks.len() >= 2, "expected 2+ chunks, got {}", chunks.len());
        // Every item is preserved across the chunks, none dropped.
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, items.len());
    }

    #[test]
    fn chunk_oversized_item_gets_its_own_chunk() {
        let items: Vec<(String, String)> = vec![
            ("short".to_string(), "r1".to_string()),
            ("x".repeat(50), "r2".to_string()),
            ("short2".to_string(), "r3".to_string()),
        ];
        let chunks = chunk_by_arg_len(&items, 10);
        // The oversized item never gets dropped for exceeding the budget —
        // it becomes a singleton chunk of its own.
        assert!(chunks.iter().any(|c| c.len() == 1 && c[0].0.len() == 50));
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, items.len());
    }
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MergePreviewDto {
    pub files: u64,
    pub conflicts: u64,
}

/// Parse `lore branch diff` output: `branchDiffChangeBegin.changesCount` is the
/// number of incoming file changes, `branchDiffConflictBegin.conflictsCount` the
/// number of conflicts.
fn merge_preview_from(events: &[LoreEvent]) -> MergePreviewDto {
    let files = events_with_tag(events, "branchDiffChangeBegin")
        .into_iter()
        .next()
        .and_then(|d| d.get("changesCount").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    let conflicts = events_with_tag(events, "branchDiffConflictBegin")
        .into_iter()
        .next()
        .and_then(|d| d.get("conflictsCount").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    MergePreviewDto { files, conflicts }
}

/// The current branch name (the merge target).
fn current_branch(repo_path: &str) -> Result<String, String> {
    let events = run_lore(&["status", "--repository", repo_path])?;
    Ok(events_with_tag(&events, "repositoryStatusRevision")
        .into_iter()
        .next()
        .and_then(|d| d.get("branchName").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_default())
}

/// Preview merging `source` into the current branch: the incoming file + conflict
/// counts, via `lore branch diff <current> --source <source>` (non-mutating).
#[tauri::command]
pub async fn lore_merge_preview(repo_path: String, source: String) -> Result<MergePreviewDto, String> {
    blocking(move || {
        let target = current_branch(&repo_path)?;
        let events = run_lore(&[
            "branch", "diff", &target, "--source", &source, "--repository", &repo_path,
        ])?;
        Ok(merge_preview_from(&events))
    })
    .await
}

/// Merge `source` into the current branch. Auto-commits when there are no
/// conflicts (the frontend only calls this for a conflict-free preview);
/// conflict resolution is a follow-up.
#[tauri::command]
pub async fn lore_merge(repo_path: String, source: String, message: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&[
            "branch", "merge", &source, "--message", &message, "--repository", &repo_path,
        ])?;
        Ok(())
    })
    .await
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MergeConflictDto {
    pub path: String,
    pub is_binary: bool,
    pub unresolved: bool,
}

/// Conflicted files during an in-progress merge: `repositoryStatusFile` entries
/// with `flagConflict`. `unresolved` = `flagConflictUnresolved` (a resolved file
/// keeps `flagConflict` true but `flagConflictUnresolved` false until committed).
fn merge_conflicts_from(events: &[LoreEvent], repo_root: &std::path::Path) -> Vec<MergeConflictDto> {
    events_with_tag(events, "repositoryStatusFile")
        .into_iter()
        .filter(|d| d.get("type").and_then(|v| v.as_str()) != Some("directory"))
        .filter(|d| d.get("flagConflict").map(json_truthy).unwrap_or(false))
        .map(|d| {
            let path = d.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            MergeConflictDto {
                is_binary: is_binary(repo_root, &path),
                unresolved: d.get("flagConflictUnresolved").map(json_truthy).unwrap_or(false),
                path,
            }
        })
        .collect()
}

/// Start merging `source` into the current branch, entering the conflict-resolution
/// state (called only when the preview shows conflicts).
#[tauri::command]
pub async fn lore_merge_start(repo_path: String, source: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&["branch", "merge", "start", &source, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// The conflicted files of the in-progress merge.
#[tauri::command]
pub async fn lore_merge_conflicts(repo_path: String) -> Result<Vec<MergeConflictDto>, String> {
    blocking(move || {
        let events = run_lore(&["status", "--scan", "--repository", &repo_path])?;
        Ok(merge_conflicts_from(&events, std::path::Path::new(&repo_path)))
    })
    .await
}

/// Resolve one conflicted file: `side` = "mine" (current branch) or "theirs"
/// (source branch). `merge resolve` resolves a relative path against the process
/// cwd, so pass an absolute path (same gotcha as lock/diff).
#[tauri::command]
pub async fn lore_merge_resolve(repo_path: String, path: String, side: String) -> Result<(), String> {
    blocking(move || {
        let sub = if side == "theirs" { "theirs" } else { "mine" };
        let abs = std::path::Path::new(&repo_path).join(&path);
        let abs_str = abs.to_string_lossy();
        run_lore(&["branch", "merge", "resolve", sub, &abs_str, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// Finalize the merge once every conflict is resolved — a plain commit that
/// records the merge revision.
#[tauri::command]
pub async fn lore_merge_commit(repo_path: String, message: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&["commit", &message, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

/// Abort the in-progress merge, restoring the pre-merge working tree.
#[tauri::command]
pub async fn lore_merge_abort(repo_path: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&["branch", "merge", "abort", "--repository", &repo_path])?;
        Ok(())
    })
    .await
}

#[cfg(test)]
mod merge_conflicts_tests {
    use super::*;
    use crate::lore::parse_events;

    const STATUS: &str = concat!(
        r#"{"tagName":"repositoryStatusFile","data":{"path":"a.txt","flagConflict":true,"flagConflictUnresolved":true}}"#, "\n",
        r#"{"tagName":"repositoryStatusFile","data":{"path":"Content/M.uasset","flagConflict":true,"flagConflictUnresolved":false}}"#, "\n",
        r#"{"tagName":"repositoryStatusFile","data":{"path":"clean.txt","flagConflict":false,"flagConflictUnresolved":false}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn lists_conflicts_with_unresolved_flag() {
        let events = parse_events(STATUS).unwrap();
        let conflicts = merge_conflicts_from(&events, std::path::Path::new(""));
        // Only the two flagConflict files; clean.txt excluded.
        assert_eq!(conflicts.len(), 2);
        let a = conflicts.iter().find(|c| c.path == "a.txt").unwrap();
        assert!(a.unresolved);
        assert!(!a.is_binary);
        let m = conflicts.iter().find(|c| c.path == "Content/M.uasset").unwrap();
        assert!(!m.unresolved);
        assert!(m.is_binary);
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;
    use crate::lore::parse_events;

    const DIFF: &str = concat!(
        r#"{"tagName":"branchDiffBegin","data":{"unused":0}}"#, "\n",
        r#"{"tagName":"branchDiffChangeBegin","data":{"changesCount":2}}"#, "\n",
        r#"{"tagName":"branchDiffChange","data":{"change":{"action":"delete","path":"notes.txt","automerged":false}}}"#, "\n",
        r#"{"tagName":"branchDiffChange","data":{"change":{"action":"add","path":"new.rs","automerged":false}}}"#, "\n",
        r#"{"tagName":"branchDiffChangeEnd","data":{"unused":0}}"#, "\n",
        r#"{"tagName":"branchDiffConflictBegin","data":{"conflictsCount":1}}"#, "\n",
        r#"{"tagName":"branchDiffConflictEnd","data":{"unused":0}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn parses_change_and_conflict_counts() {
        let events = parse_events(DIFF).unwrap();
        let preview = merge_preview_from(&events);
        assert_eq!(preview.files, 2);
        assert_eq!(preview.conflicts, 1);
    }
}

#[cfg(test)]
mod commit_files_tests {
    use super::*;
    use crate::lore::parse_events;

    const DIFF: &str = concat!(
        r#"{"tagName":"fileDiff","data":{"path":"notes.txt","patch":"@@","action":"keep"}}"#, "\n",
        r#"{"tagName":"fileDiff","data":{"path":"new.rs","patch":"@@","action":"add"}}"#, "\n",
        r#"{"tagName":"fileDiff","data":{"path":"gone.rs","patch":"@@","action":"delete"}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn maps_file_actions() {
        let events = parse_events(DIFF).unwrap();
        let files = commit_files_from(&events);
        assert_eq!(files.len(), 3);
        assert_eq!(files[0], CommitFileDto { path: "notes.txt".into(), action: "modify".into() });
        assert_eq!(files[1].action, "add");
        assert_eq!(files[2].action, "delete");
    }
}

#[cfg(test)]
mod pushed_lock_tests {
    use super::*;
    use crate::lore::parse_events;

    const DIFF: &str = concat!(
        r#"{"tagName":"fileDiff","data":{"path":"notes.txt","patch":"@@ -1 +1 @@"}}"#, "\n",
        r#"{"tagName":"fileDiff","data":{"path":"src/main.rs","patch":"@@ -1 +1 @@"}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn collects_changed_paths() {
        let events = parse_events(DIFF).unwrap();
        let paths = pushed_paths_from(&events);
        assert_eq!(paths.len(), 2);
        assert!(paths.contains("notes.txt"));
        assert!(paths.contains("src/main.rs"));
    }

    #[test]
    fn zero_revision_detection() {
        assert!(is_zero_revision(""));
        assert!(is_zero_revision("0000000000000000000000000000000000000000000000000000000000000000"));
        assert!(!is_zero_revision("a3e42aeae4e3"));
    }
}

#[cfg(test)]
mod branches_tests {
    use super::*;
    use crate::lore::parse_events;

    const SAMPLE: &str = concat!(
        r#"{"tagName":"branchListBegin","data":{"location":"local"}}"#, "\n",
        r#"{"tagName":"branchListEntry","data":{"location":"local","name":"main","latest":"a1","isCurrent":true,"archived":false}}"#, "\n",
        r#"{"tagName":"branchListEntry","data":{"location":"local","name":"old/thing","latest":"a2","isCurrent":false,"archived":true}}"#, "\n",
        r#"{"tagName":"branchListEnd","data":{"location":"local","count":2}}"#, "\n",
        r#"{"tagName":"branchListBegin","data":{"location":"remote"}}"#, "\n",
        r#"{"tagName":"branchListEntry","data":{"location":"remote","name":"main","latest":"a1","isCurrent":false,"archived":false}}"#, "\n",
        r#"{"tagName":"branchListEntry","data":{"location":"remote","name":"feature/x","latest":"a3","isCurrent":false,"archived":false}}"#, "\n",
        r#"{"tagName":"branchListEnd","data":{"location":"remote","count":2}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn unions_dedupes_and_marks_current() {
        let events = parse_events(SAMPLE).unwrap();
        let branches = branches_from(&events);
        // main (deduped local+remote) + feature/x (remote-only); archived old/thing dropped.
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, "main"); // local order first
        assert!(branches[0].current);
        assert_eq!(branches[0].location, "local"); // present locally => local wins the dedupe
        assert_eq!(branches[1].name, "feature/x");
        assert!(!branches[1].current);
        assert_eq!(branches[1].location, "remote"); // remote-only
    }

    #[test]
    fn missing_location_defaults_to_local() {
        // Older CLI without the field: no phantom Remote section.
        let sample = concat!(
            r#"{"tagName":"branchListEntry","data":{"name":"main","latest":"a1","isCurrent":true,"archived":false}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let branches = branches_from(&parse_events(sample).unwrap());
        assert_eq!(branches[0].location, "local");
    }

    #[test]
    fn parses_branch_list_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/branch_list.ndjson")).unwrap();
        let branches = branches_from(&events);
        assert!(!branches.is_empty(), "the captured fixture must list at least one branch");
        assert!(branches.iter().any(|b| b.current), "one branch is current");
        assert!(branches.iter().all(|b| b.location == "local" || b.location == "remote"));
    }

    #[test]
    fn branch_tips_maps_local_latest_to_name() {
        let events = parse_events(SAMPLE).unwrap();
        let tips = branch_tips_from(&events);
        // Only the local, non-archived tip: main@a1. Archived + remote-only excluded.
        assert_eq!(tips.get("a1").map(String::as_str), Some("main"));
        assert_eq!(tips.len(), 1);
    }
}

#[cfg(test)]
mod locks_tests {
    use super::*;
    use crate::lore::parse_events;

    const SAMPLE: &str = concat!(
        r#"{"tagName":"lockFileQueryBegin","data":{"count":1}}"#, "\n",
        r#"{"tagName":"lockFileQuery","data":{"branch":"b1","path":"notes.txt","owner":"u1","lockedAt":1783332656647}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        r#"{"tagName":"authUserInfo","data":{"id":"u1","name":"jimmy@example.com"}}"#, "\n",
    );

    #[test]
    fn resolves_owner_and_you() {
        let events = parse_events(SAMPLE).unwrap();
        let mine = locks_from(&events, "u1");
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].path, "notes.txt");
        assert_eq!(mine[0].holder, "you");

        let theirs = locks_from(&events, "someone-else");
        assert_eq!(theirs[0].holder, "jimmy@example.com");
    }
}

#[cfg(test)]
mod writes_tests {
    use super::*;

    #[test]
    fn lock_subcommand_maps_bool() {
        assert_eq!(lock_subcommand(true), "acquire");
        assert_eq!(lock_subcommand(false), "release");
    }

    #[test]
    fn commit_rejects_empty_message() {
        let err = require_commit_message("   ").unwrap_err();
        assert!(err.contains("message"), "err was {err}");
    }
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
        let status = status_from(&events, std::path::Path::new(""));
        assert!(!status.branch.is_empty());
        // Files may be empty for a clean clone; the parse must still succeed with a branch.
    }

    #[test]
    fn directory_entries_are_dropped() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":0,"isRemoteAhead":0}}"#, "\n",
            r#"{"tagName":"repositoryStatusFile","data":{"path":"Audio","size":0,"action":"add","type":"directory","flagDirty":true}}"#, "\n",
            r#"{"tagName":"repositoryStatusFile","data":{"path":"Audio/sine.wav","size":16044,"action":"add","type":"file","flagDirty":true}}"#, "\n",
            r#"{"tagName":"repositoryStatusFile","data":{"path":"legacy_no_type.txt","size":3,"action":"keep"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        let paths: Vec<&str> = s.files.iter().map(|f| f.path.as_str()).collect();
        // The directory row disappears; files (with or without a type field) stay.
        assert_eq!(paths, ["Audio/sine.wav", "legacy_no_type.txt"]);
    }

    #[test]
    fn parses_remote_flags() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionNumber":7,"revisionLocalNumber":7,"revisionRemoteNumber":7,"isLocalAhead":0,"isRemoteAhead":0,"remoteAvailable":0,"remoteAuthorized":1}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        assert_eq!(s.revision_number, 7);
        assert!(!s.remote_available);
        assert!(s.remote_authorized);
    }

    #[test]
    fn missing_remote_flags_default_online() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":0,"isRemoteAhead":0}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        assert!(s.remote_available);
        assert!(s.remote_authorized);
    }

    #[test]
    fn merge_fixture_sets_merge_in_progress() {
        let events = parse_events(include_str!("../tests/fixtures/status_merge.ndjson")).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        assert!(s.merge_in_progress);
        assert!(s.staged_pending); // a merge implies a staged state (see fixtures README)
    }

    #[test]
    fn staged_fixture_sets_staged_pending() {
        let events = parse_events(include_str!("../tests/fixtures/status_staged.ndjson")).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        assert!(s.staged_pending);
        assert!(!s.merge_in_progress);
    }

    #[test]
    fn merge_and_staged_flags() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":3,"revisionRemoteNumber":3,"isLocalAhead":false,"isRemoteAhead":false,"revisionMerged":"a3e42aeae4e3","revisionStaged":"b4f53bfbf5f4"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(sample).unwrap(), std::path::Path::new(""));
        assert!(s.merge_in_progress);
        assert!(s.staged_pending);
    }

    #[test]
    fn zero_or_absent_merge_fields_are_false() {
        // All-zero hashes = no merge/staged state.
        let zeros = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":false,"isRemoteAhead":false,"revisionMerged":"0000000000000000000000000000000000000000000000000000000000000000","revisionStaged":"0000000000000000000000000000000000000000000000000000000000000000"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(zeros).unwrap(), std::path::Path::new(""));
        assert!(!s.merge_in_progress);
        assert!(!s.staged_pending);
        // Absent fields (older CLI, no merge) must default to false too.
        let absent = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":false,"isRemoteAhead":false}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(absent).unwrap(), std::path::Path::new(""));
        assert!(!s.merge_in_progress);
        assert!(!s.staged_pending);
    }

    #[test]
    fn committed_merge_head_is_not_merge_in_progress() {
        // A merge commit keeps `revisionMerged` set (its 2nd parent) even once
        // committed AND pushed — with `revisionStaged` back to zero. That is a
        // clean state, NOT an in-progress merge: the "Merge in progress — resume"
        // chip must stay OFF, or it lingers permanently after every clean
        // sync-merge (real hash captured from the P6 catch-up sync).
        let committed = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":22,"revisionRemoteNumber":22,"isLocalAhead":false,"isRemoteAhead":false,"revisionMerged":"608dc85c8a8d0259a86d00499dbfe6302910c01f4ca218f9ca29f48ef7c81eeb","revisionStaged":"0000000000000000000000000000000000000000000000000000000000000000"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(committed).unwrap(), std::path::Path::new(""));
        assert!(!s.merge_in_progress, "a committed merge head must not read as merge-in-progress");
        assert!(!s.staged_pending);
    }

    #[test]
    fn behind_fixture_reports_a_past_revision() {
        let events = parse_events(include_str!("../tests/fixtures/status_behind.ndjson")).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        // Time-traveled: the current revision trails the local head.
        assert!(s.local_revision_number > 0);
        assert!(s.revision_number < s.local_revision_number,
            "current {} should trail local head {}", s.revision_number, s.local_revision_number);
    }

    #[test]
    fn parses_summary_from_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/status.ndjson")).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        let sum = s.summary.expect("the captured fixture carries a repositoryStatusSummary");
        assert_eq!(sum, StatusSummaryDto { adds: 1, mods: 1, dels: 0 });
    }

    #[test]
    fn missing_summary_event_is_none() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":false,"isRemoteAhead":false}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(sample).unwrap(), std::path::Path::new(""));
        assert!(s.summary.is_none());
    }

    #[test]
    fn moves_and_copies_fold_into_mods() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":false,"isRemoteAhead":false}}"#, "\n",
            r#"{"tagName":"repositoryStatusSummary","data":{"adds":2,"deletes":1,"modifies":3,"moves":1,"copies":1}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(sample).unwrap(), std::path::Path::new(""));
        assert_eq!(s.summary, Some(StatusSummaryDto { adds: 2, mods: 5, dels: 1 }));
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
        assert!(page.commits[0].when_ms > 0);
        assert_eq!(page.commits[0].parents.len(), 1); // rev 2 → one real parent (rev 1)
        assert!(page.commits[1].parents.is_empty());   // rev 1 is the root
        assert!(page.next_cursor.is_some());
        // Single-branch history → every commit stays on lane 0.
        assert!(page.commits.iter().all(|c| c.lane == 0));
    }

    // A `feature` commit (rev3) stacked on `main` (rev2, rev1). Newest first.
    const STACKED: &str = concat!(
        r#"{"tagName":"revisionHistoryEntry","data":{"revision":"r3","revisionNumber":3,"parent":["r2","0000000000000000000000000000000000000000000000000000000000000000"]}}"#, "\n",
        r#"{"tagName":"metadata","data":{"key":"branch","value":{"tagName":"context","data":"feature"}}}"#, "\n",
        r#"{"tagName":"revisionHistoryEntry","data":{"revision":"r2","revisionNumber":2,"parent":["r1","0000000000000000000000000000000000000000000000000000000000000000"]}}"#, "\n",
        r#"{"tagName":"metadata","data":{"key":"branch","value":{"tagName":"context","data":"main"}}}"#, "\n",
        r#"{"tagName":"revisionHistoryEntry","data":{"revision":"r1","revisionNumber":1,"parent":["0000000000000000000000000000000000000000000000000000000000000000","0000000000000000000000000000000000000000000000000000000000000000"]}}"#, "\n",
        r#"{"tagName":"metadata","data":{"key":"branch","value":{"tagName":"context","data":"main"}}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn stacked_branch_gets_its_own_lane() {
        let events = parse_events(STACKED).unwrap();
        let page = history_from(&events);
        let lane = |id: &str| page.commits.iter().find(|c| c.id == id).unwrap().lane;
        // The base `main` is the trunk on lane 0; the stacked `feature` commit
        // takes lane 1 so it forks visibly from main even without a merge.
        assert_eq!(lane("r1"), 0);
        assert_eq!(lane("r2"), 0);
        assert_eq!(lane("r3"), 1);
    }
}

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

#[cfg(test)]
mod op_progress_tests {
    use super::*;
    use crate::lore::parse_events;

    /// The captured fixture must yield the 3 progress ticks from a real clone
    /// (see tests/fixtures/clone_progress.ndjson): 0/0 (undiscovered), 0/202,
    /// 202/202 — the `complete` event is not a progress tick.
    #[test]
    fn maps_clone_progress_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/clone_progress.ndjson")).unwrap();
        let ticks: Vec<_> = events.iter().filter_map(op_progress_from).collect();
        assert!(!ticks.is_empty(), "the captured fixture must yield progress ticks");
        assert_eq!(ticks, vec![(0, None), (0, Some(202)), (202, Some(202))]);
    }

    #[test]
    fn maps_clone_progress_nested_under_count_and_ignores_other_events() {
        // Real wire shape (Task 13 capture): bytesTransferred/bytesTotal nested
        // under `count`, not top-level `done`/`total` as slice B had assumed.
        let sample = concat!(
            r#"{"tagName":"repositoryCloneProgress","data":{"count":{"bytesTransferred":512,"bytesTotal":2048,"discoveryComplete":true}}}"#, "\n",
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main"}}"#, "\n",
            r#"{"tagName":"repositoryCloneBegin","data":{"branch":"main"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let ticks: Vec<_> = events.iter().filter_map(op_progress_from).collect();
        assert_eq!(ticks, vec![(512, Some(2048))]);
    }

    #[test]
    fn maps_push_progress_top_level_bytes_fields() {
        // Real wire shape: branchPushFragmentProgress, top-level bytesTransferred/bytesTotal.
        let sample = concat!(
            r#"{"tagName":"branchPushFragmentProgress","data":{"complete":198,"count":559,"bytesTransferred":12131335,"bytesTotal":34073067}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let ticks: Vec<_> = events.iter().filter_map(op_progress_from).collect();
        assert_eq!(ticks, vec![(12131335, Some(34073067))]);
    }

    #[test]
    fn maps_sync_progress_bytes_update_fields() {
        // Real wire shape: revisionSyncProgress, top-level bytesUpdate/bytesUpdateTotal.
        let sample = concat!(
            r#"{"tagName":"revisionSyncProgress","data":{"fileUpdate":0,"fileUpdateTotal":1,"bytesUpdate":0,"bytesUpdateTotal":67,"discoveryComplete":true}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let ticks: Vec<_> = events.iter().filter_map(op_progress_from).collect();
        assert_eq!(ticks, vec![(0, Some(67))]);
    }

    #[test]
    fn progress_without_total_is_indeterminate() {
        // Pre-discovery sync tick: total present but 0 ("not yet known"), not "already done".
        let sample = concat!(
            r#"{"tagName":"revisionSyncProgress","data":{"fileUpdate":0,"fileUpdateTotal":0,"bytesUpdate":0,"bytesUpdateTotal":0,"discoveryComplete":false}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let ticks: Vec<_> = events.iter().filter_map(op_progress_from).collect();
        assert_eq!(ticks, vec![(0, None)]);
    }

    #[test]
    fn first_tick_is_always_emitted() {
        assert!(should_emit(None, 1, Some(100)));
    }

    #[test]
    fn final_tick_is_always_emitted_even_if_recent() {
        // Fresh Instant::now() — elapsed() is far under the 33ms throttle window.
        let last = Some(std::time::Instant::now());
        assert!(should_emit(last, 100, Some(100)));
    }

    #[test]
    fn intermediate_tick_within_window_is_suppressed() {
        let last = Some(std::time::Instant::now());
        assert!(!should_emit(last, 50, Some(100)));
    }

    #[test]
    fn intermediate_tick_with_unknown_total_is_never_treated_as_final() {
        let last = Some(std::time::Instant::now());
        assert!(!should_emit(last, 50, None));
    }

    #[test]
    fn intermediate_tick_after_window_elapses_is_emitted() {
        let last = Some(std::time::Instant::now() - std::time::Duration::from_millis(40));
        assert!(should_emit(last, 50, Some(100)));
    }
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiffLineDto {
    pub kind: String, // "add" | "del" | "context" | "hunk"
    pub text: String, // line content, WITHOUT the +/-/space prefix
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

/// Parse a hunk header `@@ -A[,B] +C[,D] @@` → `(old_start, new_start)`.
fn parse_hunk_header(line: &str) -> (u32, u32) {
    let mut old_start = 0;
    let mut new_start = 0;
    for tok in line.split_whitespace() {
        if let Some(n) = tok.strip_prefix('-') {
            old_start = n.split(',').next().and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(n) = tok.strip_prefix('+') {
            new_start = n.split(',').next().and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }
    (old_start, new_start)
}

/// Parse a unified-diff patch into structured lines with old/new line numbers
/// (GitHub-Desktop-style gutters). The `---`/`+++` file headers are dropped; the
/// `+`/`-`/space prefix is stripped from `text` (the `kind` carries the marker).
fn parse_diff(patch: &str) -> Vec<DiffLineDto> {
    let mut old_n: u32 = 0;
    let mut new_n: u32 = 0;
    let mut out = Vec::new();
    for raw in patch.lines() {
        let line = raw.trim_end_matches('\r');
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with("@@") {
            let (o, n) = parse_hunk_header(line);
            old_n = o;
            new_n = n;
            out.push(DiffLineDto { kind: "hunk".into(), text: line.to_string(), old_line: None, new_line: None });
        } else if line.starts_with('\\') {
            // Unified-diff metadata, e.g. `\ No newline at end of file` — not content.
            continue;
        } else if let Some(content) = line.strip_prefix('+') {
            out.push(DiffLineDto { kind: "add".into(), text: content.to_string(), old_line: None, new_line: Some(new_n) });
            new_n += 1;
        } else if let Some(content) = line.strip_prefix('-') {
            out.push(DiffLineDto { kind: "del".into(), text: content.to_string(), old_line: Some(old_n), new_line: None });
            old_n += 1;
        } else {
            let content = line.strip_prefix(' ').unwrap_or(line);
            out.push(DiffLineDto { kind: "context".into(), text: content.to_string(), old_line: Some(old_n), new_line: Some(new_n) });
            old_n += 1;
            new_n += 1;
        }
    }
    out
}

/// Diff of `<path>` (current revision vs working copy) as structured lines.
#[tauri::command]
pub async fn lore_diff(repo_path: String, path: String) -> Result<Vec<DiffLineDto>, String> {
    blocking(move || {
        // `lore diff` resolves a relative path against the process cwd, not
        // `--repository`, so pass an absolute path (same as `lore lock`).
        let abs = std::path::Path::new(&repo_path).join(&path);
        let abs_str = abs.to_string_lossy();
        let events = run_lore(&["diff", &abs_str, "--repository", &repo_path])?;
        let patch = events_with_tag(&events, "fileDiff")
            .into_iter()
            .next()
            .and_then(|d| d.get("patch").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        Ok(parse_diff(&patch))
    })
    .await
}

/// Diff of `<path>` between two revisions (source→target signatures) as
/// structured lines — the historical diff shown in the History preview.
#[tauri::command]
pub async fn lore_diff_revs(repo_path: String, path: String, source: String, target: String) -> Result<Vec<DiffLineDto>, String> {
    blocking(move || {
        // Same absolute-path handling as `lore_diff` — `lore diff` resolves a
        // relative path against the process cwd, not `--repository`.
        let abs = std::path::Path::new(&repo_path).join(&path);
        let abs_str = abs.to_string_lossy();
        let events = run_lore(&["diff", &abs_str, "--source", &source, "--target", &target, "--repository", &repo_path])?;
        let patch = events_with_tag(&events, "fileDiff")
            .into_iter()
            .next()
            .and_then(|d| d.get("patch").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        Ok(parse_diff(&patch))
    })
    .await
}

#[cfg(test)]
mod diff_tests {
    use super::*;

    #[test]
    fn parses_unified_patch() {
        let patch = "--- notes.txt@3\n+++ notes.txt\n@@ -1 +1,2 @@\n scratch notes\r\n+test notre\r\n";
        let lines = parse_diff(patch);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].kind, "hunk");
        assert_eq!((lines[0].old_line, lines[0].new_line), (None, None));
        assert_eq!(lines[1].kind, "context");
        assert_eq!(lines[1].text, "scratch notes"); // prefix stripped
        assert_eq!((lines[1].old_line, lines[1].new_line), (Some(1), Some(1)));
        assert_eq!(lines[2].kind, "add");
        assert_eq!(lines[2].text, "test notre"); // prefix stripped
        assert_eq!((lines[2].old_line, lines[2].new_line), (None, Some(2)));
    }

    #[test]
    fn parses_revision_range_diff_fixture() {
        let events = crate::lore::parse_events(include_str!("../tests/fixtures/file_diff_revs.ndjson")).unwrap();
        let patch = events_with_tag(&events, "fileDiff").into_iter().next().unwrap().get("patch").unwrap().as_str().unwrap();
        let lines = parse_diff(patch);
        // The captured modify diff: 2 removed lines, 3 added lines, one hunk header,
        // and NO "\ No newline" bogus lines.
        assert_eq!(lines.iter().filter(|l| l.kind == "del").count(), 2);
        assert_eq!(lines.iter().filter(|l| l.kind == "add").count(), 3);
        assert!(lines.iter().any(|l| l.kind == "hunk"));
        assert!(lines.iter().all(|l| !l.text.starts_with("\\ No newline")));
    }
}

/// Spawn a command detached, hiding the console window on Windows. The exit
/// status is deliberately not awaited — explorer.exe returns non-zero codes
/// even on success, so spawning is the only meaningful check.
fn spawn_detached(mut cmd: std::process::Command) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("failed to launch: {e}"))
}

/// The single argument for Windows `explorer`: `/select,<path>` when the file
/// exists, else its parent directory (e.g. a deleted change). Forward slashes
/// are normalized — `/select,` silently fails on them.
#[cfg(any(target_os = "windows", test))]
fn reveal_arg_windows(path: &str, exists: bool) -> String {
    let win = path.replace('/', "\\");
    if exists {
        return format!("/select,{win}");
    }
    std::path::Path::new(&win)
        .parent()
        .map(|d| d.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or(win)
}

/// Open the system file manager with the file selected (falls back to the
/// parent directory when the file is gone, e.g. a deleted change).
#[tauri::command]
pub fn os_reveal_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let exists = std::path::Path::new(&path).exists();
        let mut cmd = std::process::Command::new("explorer");
        cmd.arg(reveal_arg_windows(&path, exists));
        spawn_detached(cmd)
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("open");
        cmd.arg("-R").arg(&path);
        spawn_detached(cmd)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let p = std::path::Path::new(&path);
        let parent = p.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone());
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(parent);
        spawn_detached(cmd)
    }
}

/// Open the file with its default application.
#[tauri::command]
pub fn os_open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let win = path.replace('/', "\\");
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/c", "start", "", &win]);
        spawn_detached(cmd)
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("open");
        cmd.arg(&path);
        spawn_detached(cmd)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(&path);
        spawn_detached(cmd)
    }
}

/// Pure check behind `os_path_exists` (testable without Tauri).
fn path_exists_impl(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

/// Does this directory still exist? Drives the "Missing" state of the repo list.
#[tauri::command]
pub fn os_path_exists(path: String) -> bool {
    path_exists_impl(&path)
}

/// Re-register a repository after the user moved its folder on disk. The clone's
/// local metadata already lets `status` work at the new path (verified — constat
/// (a) of the Task 2 capture); `repository update-path` is a best-effort fix of
/// the server-side instance registry (clears the "stale" flag), so its failure
/// is non-fatal. The status at the new path is the real proof of life.
#[tauri::command]
pub async fn lore_update_path(new_path: String) -> Result<(), String> {
    blocking(move || {
        let _ = run_lore(&["repository", "update-path", "--repository", &new_path]);
        run_lore(&["status", "--repository", &new_path])?;
        Ok(())
    })
    .await
}

#[cfg(test)]
mod repo_health_tests {
    use super::*;

    #[test]
    fn path_exists_reports_real_directories() {
        let dir = std::env::temp_dir();
        assert!(path_exists_impl(dir.to_str().unwrap()));
        assert!(!path_exists_impl(dir.join("p3-definitely-missing-xyz").to_str().unwrap()));
    }
}

#[cfg(test)]
mod reveal_arg_tests {
    use super::reveal_arg_windows;

    #[test]
    fn existing_file_selects_with_backslashes() {
        assert_eq!(
            reveal_arg_windows("C:/repo/Content/T_Icon.png", true),
            r"/select,C:\repo\Content\T_Icon.png"
        );
    }

    #[test]
    fn missing_file_falls_back_to_parent() {
        assert_eq!(reveal_arg_windows("C:/repo/Docs/gone.md", false), r"C:\repo\Docs");
    }

    #[test]
    fn missing_file_without_parent_keeps_path() {
        assert_eq!(reveal_arg_windows("gone.md", false), "gone.md");
    }
}
