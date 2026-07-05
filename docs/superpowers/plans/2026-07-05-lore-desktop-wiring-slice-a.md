# Lore Desktop — Wiring Slice A Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the in-memory mock with a real backend for a thin, read-only vertical slice — `isAuthenticated`, `signIn`, `getStatus`, `getHistory` — proving the pipeline `Tauri command → lore … --json → parse NDJSON in Rust → LoreApi → live UI` against the real `lore.example.com` server.

**Architecture:** Each wired `LoreApi` method is a typed Rust `#[tauri::command]` (`src-tauri`) that runs the matching `lore` subcommand with `--json` + `--repository <path>`, parses the NDJSON stream (`{"tagName","data"}` lines terminated by `complete`) with `serde`, and returns a struct serialized to the exact TS shape. `src/lib/api.ts` picks the Tauri implementation when running inside Tauri, else the existing mock, so browser-only UI work still runs. The `getHistory` contract grows real pagination (`length` + `cursor`), applied to the mock too.

**Tech Stack:** Tauri v2 (Rust `std::process::Command`, `serde`/`serde_json`), Svelte 5 + `@tauri-apps/api` (`invoke`), the bundled-on-PATH `lore 0.8.3` CLI.

**Ground-truth references (read-only, do not modify):**
- Event enum + JSON tagging: `D:\GitHub\lore\lore-revision\src\event.rs` (`LoreEvent` = `#[serde(tag="tagName", content="data", rename_all="camelCase")]`).
- Status shapes: `D:\GitHub\lore\lore-revision\src\repository\status.rs`.
- History shapes: `D:\GitHub\lore\lore-revision\src\revision\history.rs`.
- The current mock contract: `src/lib/types.ts`, `src/lib/mock.ts`.

---

## File structure

Rust (`src-tauri/`):
- Create `src/lore.rs` — process runner + NDJSON line parser (pure, unit-tested).
- Create `src/commands.rs` — the four `#[tauri::command]`s, their DTO structs (serde → TS shapes), and per-command event→DTO mapping. Unit-tested against captured fixtures.
- Create `tests/fixtures/*.ndjson` — real captured `lore … --json` output (Task 2).
- Modify `src/lib.rs` — declare the modules and register the commands in `invoke_handler`.

Frontend (`src/`):
- Modify `package.json` — add `@tauri-apps/api`.
- Modify `src/lib/types.ts` — new `HistoryPage` + `getHistory(repoPath, length, cursor?)` signature.
- Modify `src/lib/mock.ts` — `getHistory` returns a `HistoryPage`.
- Create `src/lib/tauri.ts` — the real `LoreApi` (invoke for the 4 wired methods, delegate the rest to `mock`).
- Modify `src/lib/api.ts` — choose `tauriApi` when inside Tauri, else `mock`.
- Modify `src/lib/History.svelte` — consume `HistoryPage`, fetch the next page when the scroll window nears the loaded end.

---

## Task 1: Frontend `@tauri-apps/api` + invoke smoke path

**Files:**
- Modify: `package.json`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the frontend Tauri API package**

Run: `npm --prefix C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop install @tauri-apps/api@^2`
Expected: `package.json` gains `"@tauri-apps/api"` under dependencies; no errors.

- [ ] **Step 2: Create a trivial command to prove `invoke` works**

Create `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}
```

- [ ] **Step 3: Register the module + command**

Modify `src-tauri/src/lib.rs` — add `mod commands;` at the top, and add the invoke handler to the builder (chain before `.run(...)`):

```rust
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![commands::ping])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
```

- [ ] **Step 4: Verify the Rust builds**

Run: `cargo build --manifest-path C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop/src-tauri/Cargo.toml`
Expected: `Finished` with no errors.

- [ ] **Step 5: Commit**

```bash
git add package.json package-lock.json src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(wiring): add @tauri-apps/api + ping command scaffold"
```

---

## Task 2: Capture real `--json` fixtures (⚠️ user-assisted: needs login)

**Why:** The exact `action` encoding (`repositoryStatusFile.action`) and the `metadata` event key/value encoding are only knowable from real output. These fixtures are the test oracle for the parsers.

**Files:**
- Create: `src-tauri/tests/fixtures/auth_list.ndjson`
- Create: `src-tauri/tests/fixtures/status.ndjson`
- Create: `src-tauri/tests/fixtures/history.ndjson`

- [ ] **Step 1: Ensure a signed-in session + a local repo (user action)**

The current stored token is expired. In a terminal the user runs:
```
lore login --auth-url https://lore.example.com:8081 lore://lore.example.com:41337
```
(completes the browser SSO), then clones a repo to test against, e.g.:
```
lore clone lore://lore.example.com:41337/<repoId> C:/Users/jimmy/lore-test-repo
```
Record the clone path as `<REPO>` for the following steps.

- [ ] **Step 2: Capture the three fixtures**

Run (PowerShell), replacing `<REPO>`:
```
$lore = "C:\Users\jimmy\bin\lore.exe"
& $lore auth list --json                                   > src-tauri/tests/fixtures/auth_list.ndjson
& $lore status --repository <REPO> --json                  > src-tauri/tests/fixtures/status.ndjson
& $lore history 20 --repository <REPO> --json              > src-tauri/tests/fixtures/history.ndjson
```
Expected: three non-empty files, each ending with a `{"tagName":"complete","data":{"status":0}}` line.

- [ ] **Step 3: Record the pinned encodings in a comment block**

Open the three fixtures and note, at the top of `src-tauri/tests/fixtures/README.md` (create it), the concrete values discovered: the JSON type/value of `repositoryStatusFile.action` (integer vs string, and the value for each of add/keep/delete/move/copy), and the exact `metadata` event shape emitted between `revisionHistoryEntry` events (the `key` strings for message/author/date/branch and how `value` is encoded). Later tasks reference these.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/fixtures/
git commit -m "test(wiring): capture real lore --json fixtures for auth/status/history"
```

---

## Task 3: NDJSON process runner + line parser (Rust)

**Files:**
- Create: `src-tauri/src/lore.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod lore;`)
- Test: inline `#[cfg(test)]` in `src-tauri/src/lore.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/lore.rs` with the parser + a test using a small literal sample:

```rust
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct LoreEvent {
    #[serde(rename = "tagName")]
    pub tag_name: String,
    pub data: Value,
}

/// Parse an NDJSON stream (one JSON object per line). Non-JSON lines are
/// skipped. Returns every event in order. Errors only on a malformed stream
/// where no `complete` event is present.
pub fn parse_events(stdout: &str) -> Result<Vec<LoreEvent>, String> {
    let mut events = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(ev) = serde_json::from_str::<LoreEvent>(line) {
            events.push(ev);
        }
    }
    if !events.iter().any(|e| e.tag_name == "complete") {
        return Err("lore did not emit a completion event".to_string());
    }
    Ok(events)
}

/// Return Err(message) if the terminal `complete` status is non-zero or an
/// `error` event was emitted.
pub fn check_ok(events: &[LoreEvent]) -> Result<(), String> {
    if let Some(err) = events.iter().find(|e| e.tag_name == "error") {
        return Err(err.data.to_string());
    }
    let status = events
        .iter()
        .rev()
        .find(|e| e.tag_name == "complete")
        .and_then(|e| e.data.get("status"))
        .and_then(|s| s.as_i64())
        .unwrap_or(-1);
    if status != 0 {
        return Err(format!("lore exited with status {status}"));
    }
    Ok(())
}

pub fn events_with_tag<'a>(events: &'a [LoreEvent], tag: &str) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|e| e.tag_name == tag)
        .map(|e| &e.data)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_checks_ok() {
        let sample = concat!(
            r#"{"tagName":"authIdentity","data":{"userId":"u1"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        assert_eq!(events_with_tag(&events, "authIdentity").len(), 1);
        assert!(check_ok(&events).is_ok());
    }

    #[test]
    fn non_zero_status_is_error() {
        let sample = concat!(
            r#"{"tagName":"error","data":{"errorInner":"nope"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":1}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        assert!(check_ok(&events).is_err());
    }
}
```

Add `mod lore;` to `src-tauri/src/lib.rs`.

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test --manifest-path C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop/src-tauri/Cargo.toml lore::`
Expected: 2 passed.

- [ ] **Step 3: Add the process runner (no test — thin OS wrapper)**

Append to `src-tauri/src/lore.rs`:

```rust
use std::process::Command;

/// Run `lore <args> --json`, capturing stdout. `--json` is appended here so
/// callers pass only the subcommand + options.
pub fn run_lore(args: &[&str]) -> Result<Vec<LoreEvent>, String> {
    let output = Command::new("lore")
        .args(args)
        .arg("--json")
        .output()
        .map_err(|e| format!("failed to launch lore: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let events = parse_events(&stdout)?;
    check_ok(&events)?;
    Ok(events)
}
```

- [ ] **Step 4: Build to confirm it compiles**

Run: `cargo build --manifest-path C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop/src-tauri/Cargo.toml`
Expected: `Finished`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lore.rs src-tauri/src/lib.rs
git commit -m "feat(wiring): NDJSON parser + lore process runner"
```

---

## Task 4: `lore_is_authenticated` command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: inline `#[cfg(test)]` in `src-tauri/src/commands.rs` using `tests/fixtures/auth_list.ndjson`

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/commands.rs` (parsing split out so it is testable without spawning a process):

```rust
use crate::lore::{events_with_tag, run_lore, LoreEvent};

/// True iff any stored identity has an `expires` in the future.
fn is_authenticated_from(events: &[crate::lore::LoreEvent], now_ms: i64) -> bool {
    events_with_tag(events, "authIdentity").iter().any(|d| {
        d.get("expires").and_then(|e| e.as_i64()).map(|exp| exp > now_ms).unwrap_or(false)
    })
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
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --manifest-path C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop/src-tauri/Cargo.toml commands::`
Expected: PASS (a captured identity with `expires > 0` exists).

- [ ] **Step 3: Add the command**

Add to `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn lore_is_authenticated() -> Result<bool, String> {
    let events = run_lore(&["auth", "list"])?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(is_authenticated_from(&events, now_ms))
}
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(wiring): lore_is_authenticated via auth list"
```

---

## Task 5: `lore_status` command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: inline, using `tests/fixtures/status.ndjson`

- [ ] **Step 1: Write the failing test + DTOs + mapping**

Add to `src-tauri/src/commands.rs`. Fill the `action` match arms and the ahead/behind fields from the values pinned in Task 2's README.

```rust
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
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --manifest-path C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop/src-tauri/Cargo.toml status_tests`
Expected: PASS (branch parsed from the fixture).

- [ ] **Step 3: Add the command**

```rust
#[tauri::command]
pub fn lore_status(repo_path: String) -> Result<StatusResultDto, String> {
    let events = run_lore(&["status", "--repository", &repo_path])?;
    Ok(status_from(&events))
}
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(wiring): lore_status via status --json"
```

---

## Task 6: `lore_history` command (entry+metadata correlation, pagination)

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: inline, using `tests/fixtures/history.ndjson`

- [ ] **Step 1: Write the failing test + DTOs + correlation**

Add to `src-tauri/src/commands.rs`. Use the metadata `key` names pinned in Task 2's README for message/author/date; the code below reads them defensively.

```rust
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

/// Walk the event stream in order. Each `revisionHistoryEntry` starts a commit;
/// the `metadata` events that follow (until the next entry) fill message/author/
/// date. `head` is the branch name from the initial `revisionHistory` header,
/// attached to the first commit only.
fn history_from(events: &[LoreEvent]) -> HistoryPage {
    let head_branch = events_with_tag(events, "revisionHistory")
        .into_iter().next()
        .and_then(|d| d.get("branch")).and_then(|v| v.as_str()).map(String::from);

    let mut commits: Vec<CommitDto> = Vec::new();
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
                    head: if commits.is_empty() { head_branch.clone() } else { None },
                    id, rev, parents,
                    message: String::new(), author: String::new(), when: String::new(),
                    adds: 0, mods: 0, dels: 0, lane: 0, files: Vec::new(),
                });
            }
            "metadata" => {
                if let Some(c) = commits.last_mut() {
                    let key = ev.data.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    let val = ev.data.get("value").map(json_scalar_string).unwrap_or_default();
                    match key {
                        "message" => c.message = val,
                        "creator" | "committer" => { if c.author.is_empty() { c.author = val } }
                        "date" => c.when = val,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    let next_cursor = commits.last().map(|c| c.id.clone());
    HistoryPage { commits, next_cursor }
}

/// Extract a scalar string from a `metadata` value, which may be a bare string,
/// a number, or a tagged `{ "string": "..." }` / `{ "numeric": n }` object.
fn json_scalar_string(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() { return s.to_string(); }
    if let Some(n) = v.as_i64() { return n.to_string(); }
    if let Some(obj) = v.as_object() {
        if let Some(inner) = obj.values().next() { return json_scalar_string(inner); }
    }
    String::new()
}

#[cfg(test)]
mod history_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_history_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/history.ndjson")).unwrap();
        let page = history_from(&events);
        assert!(!page.commits.is_empty());
        assert!(page.commits.iter().all(|c| !c.id.is_empty()));
        assert!(page.next_cursor.is_some());
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test --manifest-path C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop/src-tauri/Cargo.toml history_tests`
Expected: PASS. If `message`/`author`/`when` come back empty, correct the `key` match arms to the strings recorded in Task 2's README and re-run.

- [ ] **Step 3: Add the command**

```rust
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
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(wiring): lore_history with entry+metadata correlation and cursor paging"
```

---

## Task 7: `lore_sign_in` command

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Add the command (interactive; no NDJSON)**

```rust
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
```

- [ ] **Step 2: Build to confirm it compiles**

Run: `cargo build --manifest-path C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop/src-tauri/Cargo.toml`
Expected: `Finished`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(wiring): lore_sign_in via lore login"
```

---

## Task 8: Register all commands

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Extend the invoke handler**

Replace the `.invoke_handler(...)` line in `src-tauri/src/lib.rs` with:

```rust
    .invoke_handler(tauri::generate_handler![
        commands::ping,
        commands::lore_is_authenticated,
        commands::lore_sign_in,
        commands::lore_status,
        commands::lore_history,
    ])
```

- [ ] **Step 2: Build**

Run: `cargo build --manifest-path C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop/src-tauri/Cargo.toml`
Expected: `Finished`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(wiring): register lore commands in invoke handler"
```

---

## Task 9: `getHistory` contract → paginated (types + mock)

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/mock.ts`
- Test: `src/lib/mock.test.ts`

- [ ] **Step 1: Update the type**

In `src/lib/types.ts`, add above `LoreApi`:

```ts
export interface HistoryPage {
  commits: Commit[]
  nextCursor: string | null
}
```

Change the `LoreApi` method signature from `getHistory(repoPath: string): Promise<Commit[]>` to:

```ts
  getHistory(repoPath: string, length: number, cursor?: string): Promise<HistoryPage>
```

- [ ] **Step 2: Write the failing mock test**

Add to `src/lib/mock.test.ts`:

```ts
test('getHistory paginates by length + cursor', async () => {
  const p1 = await mock.getHistory('game-main', 10)
  expect(p1.commits).toHaveLength(10)
  expect(p1.nextCursor).not.toBeNull()
  const p2 = await mock.getHistory('game-main', 10, p1.nextCursor!)
  expect(p2.commits[0].id).not.toBe(p1.commits[0].id)
})
```

- [ ] **Step 3: Run to verify it fails**

Run: `npm --prefix C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop test`
Expected: FAIL (getHistory returns an array, `.commits` undefined).

- [ ] **Step 4: Update the mock**

In `src/lib/mock.ts`, replace `getHistory` with:

```ts
  async getHistory(_repoPath: string, length: number, cursor?: string) {
    await delay(280)
    const start = cursor ? BIG_HISTORY.findIndex((c) => c.id === cursor) + 1 : 0
    const commits = BIG_HISTORY.slice(start, start + length)
    const nextIndex = start + length
    return { commits, nextCursor: nextIndex < BIG_HISTORY.length ? commits[commits.length - 1].id : null }
  },
```

- [ ] **Step 5: Run to verify it passes**

Run: `npm --prefix C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop test`
Expected: PASS (all tests green).

- [ ] **Step 6: Commit**

```bash
git add src/lib/types.ts src/lib/mock.ts src/lib/mock.test.ts
git commit -m "feat(wiring): getHistory returns a paginated HistoryPage"
```

---

## Task 10: `tauri.ts` real API + `api.ts` selection

**Files:**
- Create: `src/lib/tauri.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Create the Tauri-backed API**

Create `src/lib/tauri.ts` (invoke the 4 wired methods; delegate everything else to the mock so unwired screens keep working):

```ts
import { invoke } from '@tauri-apps/api/core'
import { mock } from './mock'
import type { HistoryPage, LoreApi, StatusResult } from './types'

export const tauriApi: LoreApi = {
  ...mock,
  isAuthenticated: () => invoke<boolean>('lore_is_authenticated'),
  signIn: (serverUrl, authUrlOverride) =>
    invoke<void>('lore_sign_in', { serverUrl, authUrl: authUrlOverride ?? null }),
  getStatus: (repoPath) => invoke<StatusResult>('lore_status', { repoPath }),
  getHistory: (repoPath, length, cursor) =>
    invoke<HistoryPage>('lore_history', { repoPath, length, cursor: cursor ?? null }),
}
```

- [ ] **Step 2: Select the implementation in `api.ts`**

Replace `src/lib/api.ts` body with:

```ts
import { mock } from './mock'
import { tauriApi } from './tauri'
import type { LoreApi } from './types'

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const api: LoreApi = inTauri ? tauriApi : mock
export * from './types'
```

- [ ] **Step 3: Verify types + browser build still pass**

Run: `npm --prefix C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop run check`
Expected: 0 errors, 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add src/lib/tauri.ts src/lib/api.ts
git commit -m "feat(wiring): Tauri-backed api.ts (real 4 methods, mock fallback)"
```

---

## Task 11: History fetch-more on scroll

**Files:**
- Modify: `src/lib/History.svelte`

- [ ] **Step 1: Load the first page + append on scroll near the end**

In `src/lib/History.svelte`, replace the single `api.getHistory(repoPath)` load with a paged loader. Add state and change the effect + `onScroll`:

```ts
  const PAGE = 200
  let nextCursor = $state<string | null | undefined>(undefined) // undefined = not loaded, null = end
  let loadingMore = $state(false)

  $effect(() => {
    const repoPath = session.config.currentRepo
    if (!repoPath) return
    loading = true
    commits = []
    api.getHistory(repoPath, PAGE).then((page) => {
      commits = page.commits
      nextCursor = page.nextCursor
      if (page.commits.length && (selectedId === null || !page.commits.some((c) => c.id === selectedId)))
        selectedId = page.commits[0].id
      loading = false
    })
  })

  async function loadMore() {
    const repoPath = session.config.currentRepo
    if (!repoPath || loadingMore || !nextCursor) return
    loadingMore = true
    const page = await api.getHistory(repoPath, PAGE, nextCursor)
    commits = [...commits, ...page.commits]
    nextCursor = page.nextCursor
    loadingMore = false
  }
```

Change `onScroll` to trigger `loadMore` when near the loaded end:

```ts
  function onScroll() {
    if (!glistEl) return
    scrollTop = glistEl.scrollTop
    if (glistEl.scrollTop + glistEl.clientHeight > commits.length * ROW_H - viewH * 2) loadMore()
  }
```

- [ ] **Step 2: Verify types + browser build (mock still returns a HistoryPage)**

Run: `npm --prefix C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop run check`
Expected: 0 errors.

Run: `npm --prefix C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop run dev` and, via the preview tools, open History and scroll — it keeps loading pages (mock still drives it in the browser).
Expected: smooth infinite scroll, no console errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/History.svelte
git commit -m "feat(wiring): History loads pages and fetches more on scroll"
```

---

## Task 12: End-to-end verification (⚠️ user-assisted: `tauri dev` + login)

**Files:** none (verification only)

- [ ] **Step 1: Run the desktop app**

Run: `npx --prefix C:/Users/jimmy/Documents/SoonerOrLater/lore-desktop tauri dev`
(First run compiles the frontend + Rust; a desktop window opens.)

- [ ] **Step 2: Verify the real pipeline**

- Sign in with `lore://lore.example.com:41337` (+ auth-url override `https://lore.example.com:8081`); the browser SSO completes and the app advances past sign-in.
- Pick the cloned repo (`<REPO>` from Task 2); the title bar shows the real branch, and Sync/Push counts reflect real ahead/behind.
- History shows real commits (real messages/authors/revisions) and paginates on scroll.

- [ ] **Step 3: Capture proof + note follow-ups**

Screenshot the running app. File follow-ups: real per-commit `lane` layout (currently linear), per-commit file stats (`adds/mods/dels`/`files`) via `revision info`, and the `isBinary`/binary-compare accuracy.

---

## Self-review

- **Spec coverage:** architecture (Tasks 1,3,8,10) · NDJSON parsing (Task 3) · the 4 command mappings (Tasks 4–7) · pagination (Tasks 6,9,11) · graph fidelity resolved to "parents available, lanes deferred" (Task 6 + Task 12 follow-up) · auth expiry (Task 4) · error handling (`check_ok`, Task 3) · testing (fixtures Task 2, Rust unit tests Tasks 3–6, E2E Task 12). Covered.
- **Placeholder scan:** two values are pinned by the captured fixtures rather than guessed — the `action` wire encoding (Task 5) and the `metadata` key strings (Task 6). Both are resolved in Task 2 (README) before the dependent tests run, and the tests fail loudly until the mapping matches the real output. This is a deliberate capture-then-assert, not an unfilled placeholder.
- **Type consistency:** `HistoryPage { commits, nextCursor }` and `getHistory(repoPath, length, cursor?)` are used identically in `types.ts` (Task 9), `mock.ts` (Task 9), `tauri.ts` (Task 10), and `History.svelte` (Task 11). Rust DTOs use `#[serde(rename_all="camelCase")]` so `localAhead`/`isBinary`/`nextCursor` match the TS names.
