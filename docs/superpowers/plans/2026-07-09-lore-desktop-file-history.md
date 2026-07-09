# File History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-07-09-lore-desktop-file-history-design.md`

**Goal:** Per-asset revision timeline (who, when, message, size) in FilePreview.

---

### Task 1: Rust — `lore_file_history`

**Files:** Modify `src-tauri/src/commands.rs` (after `lore_commit_files`), `src-tauri/src/lib.rs`

- [x] **Step 1:** Add DTO + parser + resolver + command:

```rust
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
```

`lib.rs`: register `commands::lore_file_history`.

- [x] **Step 2:** `cargo test` PASS → commit `feat(files): per-file revision history command with author resolution`.

---

### Task 2: Front — types/API/mock + section FilePreview

**Files:** Modify `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/FilePreview.svelte`

- [x] **Step 1:** types.ts :

```ts
export interface FileRevision {
  revision: string
  revisionNumber: number
  action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
  size: number
  message: string
  author: string    // email (resolved) or raw id; 'you' mapping is UI-side
  when: string      // relative time
  whenMs: number    // absolute, for the tooltip
}
```

`LoreApi`: `/** Revision timeline of one file (newest first). */ getFileHistory(repoPath: string, path: string): Promise<FileRevision[]>`.

- [x] **Step 2:** tauri.ts : `getFileHistory: (repoPath, path) => invoke<FileRevision[]>('lore_file_history', { repoPath, path }),` — mock.ts : trois révisions synthétiques (modify 'you' 2 h, modify maya 3 j, add maya 7 j) avec tailles décroissantes ; `whenMs: Date.now() - …`.

- [x] **Step 3:** FilePreview — état `fileHistory/fhLoading/fhError/lastFhPath`, effet anti-course sur `file` (tous types de fichiers), section sous `<dl class="meta">` :

```svelte
      <div class="fhhead">History{#if fileHistory.length} · {fileHistory.length} revisions{/if}</div>
      {#if fhLoading}
        <p class="fhnote muted">Loading history…</p>
      {:else if fhError}
        <p class="fhnote muted">Couldn't load file history.</p>
      {:else if fileHistory.length === 0}
        <p class="fhnote muted">No committed revisions yet.</p>
      {:else}
        <ul class="fhl">
          {#each fileHistory.slice(0, 30) as r (r.revision)}
            <li>
              <span class="tag {glyph[r.action]?.c}">{glyph[r.action]?.v ?? '?'}</span>
              <span class="frev">#{r.revisionNumber}</span>
              <span class="fmsg" title={r.message}>{r.message}</span>
              <span class="fwho">{authorLabel(r.author)}</span>
              <span class="fwhen" title={new Date(r.whenMs).toLocaleString()}>{r.when}</span>
              <span class="fsize">{fmtSize(r.size)}</span>
            </li>
          {/each}
        </ul>
        {#if fileHistory.length > 30}<p class="fhnote muted">…and {fileHistory.length - 30} more revisions</p>{/if}
      {/if}
```

avec `glyph` (map action → glyphe coloré, comme Changes), `authorLabel` (`you` si email = identité, sinon partie locale), CSS lignes compactes.

- [x] **Step 4:** `npm run check && npm test` PASS → commit `feat(preview): per-asset revision history in the file panel`.

---

### Task 3: Vérification

- [x] Mock : sélection d'un fichier → 3 révisions, auteurs 'you'/maya, tooltip date absolue.
- [x] App réelle : `README.md` de `lore-test-repo` → révisions réelles avec messages, auteur résolu en email, tailles.
- [x] Suites complètes PASS ; commit fixes.
