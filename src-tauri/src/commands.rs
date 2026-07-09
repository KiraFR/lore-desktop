use crate::lore::{events_with_tag, run_lore, LoreEvent};

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

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusResultDto {
    pub branch: String,
    pub local_ahead: u64,
    pub remote_ahead: u64,
    pub revision_number: u64,
    pub remote_available: bool,
    pub remote_authorized: bool,
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

    let files = events_with_tag(events, "repositoryStatusFile").into_iter().map(|d| {
        let path = d.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        ChangedFileDto {
            is_binary: is_binary(repo_root, &path),
            action: d.get("action").map(map_action).unwrap_or_else(|| "modify".into()),
            size: d.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
            path,
        }
    }).collect();

    StatusResultDto { branch, local_ahead, remote_ahead, revision_number, remote_available, remote_authorized, files }
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

/// Clone `<server_url>/<repo_id>` into `<dest_parent>/<repo_name>` and return the
/// created path. `run_lore` blocks until the clone finishes and errors on a
/// non-zero terminal `complete.status` — the picker shows that as a toast.
#[tauri::command]
pub async fn lore_clone(
    server_url: String,
    repo_id: String,
    repo_name: String,
    dest_parent: String,
) -> Result<String, String> {
    blocking(move || {
        let (url, path) = build_clone_args(&server_url, &repo_id, &repo_name, &dest_parent);
        run_lore(&["clone", &url, &path])?;
        Ok(path)
    })
    .await
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
pub async fn lore_push(repo_path: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&["push", "--repository", &repo_path])?;
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
pub async fn lore_sync(repo_path: String) -> Result<(), String> {
    blocking(move || {
        run_lore(&["sync", "--repository", &repo_path])?;
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
}

/// Union of `branchListEntry` events (which stream once per location, local then
/// remote) deduped by name. `current` folds `isCurrent` across every entry for a
/// name (only local entries carry it); archived branches are dropped. First-seen
/// order is preserved, so local branches come first and remote-only ones append.
fn branches_from(events: &[LoreEvent]) -> Vec<BranchDto> {
    let mut order: Vec<String> = Vec::new();
    let mut current: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for d in events_with_tag(events, "branchListEntry") {
        if d.get("archived").map(json_truthy).unwrap_or(false) {
            continue;
        }
        let name = match d.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };
        if d.get("isCurrent").map(json_truthy).unwrap_or(false) {
            current.insert(name.clone());
        }
        if seen.insert(name.clone()) {
            order.push(name);
        }
    }
    order
        .into_iter()
        .map(|name| BranchDto { current: current.contains(&name), name })
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
        assert_eq!(branches[1].name, "feature/x");
        assert!(!branches[1].current);
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
}
