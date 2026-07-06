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
    // `--scan` reconciles the working tree so edits/adds/deletes show up in the
    // Changes view (a read-only status without it misses unstaged working changes,
    // which would leave the Commit button disabled). Non-destructive: it refreshes
    // dirty flags on the local working copy, it does not touch file contents.
    let events = run_lore(&["status", "--scan", "--repository", &repo_path])?;
    Ok(status_from(&events))
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
pub fn lore_repositories(server_url: String) -> Result<Vec<RepoEntryDto>, String> {
    let events = run_lore(&["repository", "list", &server_url])?;
    Ok(repositories_from(&events))
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
pub fn lore_clone(
    server_url: String,
    repo_id: String,
    repo_name: String,
    dest_parent: String,
) -> Result<String, String> {
    let (url, path) = build_clone_args(&server_url, &repo_id, &repo_name, &dest_parent);
    run_lore(&["clone", &url, &path])?;
    Ok(path)
}

#[tauri::command]
pub fn lore_sign_in(server_url: String, auth_url: Option<String>) -> Result<(), String> {
    let mut cmd = std::process::Command::new("lore");
    cmd.arg("login");
    if let Some(ref a) = auth_url {
        cmd.arg("--auth-url").arg(a);
    }
    cmd.arg(&server_url);
    let status = cmd.status().map_err(|e| format!("failed to launch lore login: {e}"))?;
    if status.success() { Ok(()) } else { Err("sign-in failed or was cancelled".to_string()) }
}

/// `lore lock` subcommand for a lock/unlock toggle.
fn lock_subcommand(lock: bool) -> &'static str {
    if lock {
        "acquire"
    } else {
        "release"
    }
}

/// Stage the whole working tree then commit it. Selective staging is a follow-up;
/// this commits everything, matching the current UI (checkboxes are decorative).
#[tauri::command]
pub fn lore_commit(repo_path: String, message: String) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("commit message is required".to_string());
    }
    run_lore(&["stage", ".", "--scan", "--repository", &repo_path])?;
    run_lore(&["commit", &message, "--repository", &repo_path])?;
    Ok(())
}

#[tauri::command]
pub fn lore_push(repo_path: String) -> Result<(), String> {
    run_lore(&["push", "--repository", &repo_path])?;
    Ok(())
}

/// Plain `lore sync` — pulls/merges the remote into the local branch
/// non-destructively (NO `--reset`, which would discard local modifications).
#[tauri::command]
pub fn lore_sync(repo_path: String) -> Result<(), String> {
    run_lore(&["sync", "--repository", &repo_path])?;
    Ok(())
}

#[tauri::command]
pub fn lore_set_lock(repo_path: String, path: String, lock: bool) -> Result<(), String> {
    // `lore lock` resolves a relative path against the process cwd, not `--repository`,
    // so build an absolute path inside the repo.
    let abs = std::path::Path::new(&repo_path).join(&path);
    let abs_str = abs.to_string_lossy();
    run_lore(&["lock", lock_subcommand(lock), &abs_str, "--repository", &repo_path])?;
    Ok(())
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
pub fn lore_locks(repo_path: String) -> Result<Vec<LockEntryDto>, String> {
    let me = current_user_id();
    let events = run_lore(&["lock", "query", "--repository", &repo_path])?;
    Ok(locks_from(&events, &me))
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
pub fn lore_branches(repo_path: String) -> Result<Vec<BranchDto>, String> {
    let events = run_lore(&["branch", "list", "--repository", &repo_path])?;
    Ok(branches_from(&events))
}

#[tauri::command]
pub fn lore_switch_branch(repo_path: String, name: String) -> Result<(), String> {
    run_lore(&["branch", "switch", &name, "--repository", &repo_path])?;
    Ok(())
}

/// `lore branch create` makes the branch from the current latest and auto-switches
/// to it, so no separate switch is needed. The base is always the current HEAD.
#[tauri::command]
pub fn lore_create_branch(repo_path: String, name: String) -> Result<(), String> {
    run_lore(&["branch", "create", &name, "--repository", &repo_path])?;
    Ok(())
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
        let err = lore_commit("C:/nonexistent-repo".into(), "   ".into()).unwrap_err();
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
pub fn lore_diff(repo_path: String, path: String) -> Result<Vec<DiffLineDto>, String> {
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
