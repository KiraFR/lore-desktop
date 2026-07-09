# P0 Pass Implementation Plan — bugs, identity, offline, binary detection

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-07-09-lore-desktop-p0-fixes-design.md`

**Goal:** Fix the five audit bugs and add real identity (avatar menu), an offline/session indicator, and robust binary detection.

**Architecture:** Backend-first: four independent Rust changes (new commands, status enrichment, pagination fix, binary sniff), then TS API/types/mock parity, then UI wiring (Changes, AvatarMenu/TitleBar, History, StatusBar, Locks, FilePreview). Pure logic goes in plain-TS modules (`identity.ts`, `commitMessage.ts`) because vitest has no Svelte plugin.

**Tech Stack:** Rust (Tauri 2), Svelte 5 runes, TypeScript, vitest, cargo test.

**Commands** (from repo root): `npm run check` · `npm test` · `cargo test --manifest-path src-tauri/Cargo.toml`

**Spec deviation (intentional):** FilePreview already has a Lock/Unlock toggle in its details `<dl>` (`FilePreview.svelte:128-136`) — the spec's §3 FilePreview part is already satisfied; no header button is added (YAGNI). Only the Locks-view picker is new.

---

### Task 1: Rust — `lore_identity` + `lore_sign_out`

**Files:**
- Modify: `src-tauri/src/commands.rs` (add after `lore_sign_in`, ~line 370)
- Modify: `src-tauri/src/lib.rs:48` (register)

- [x] **Step 1: Add the commands + parser + test**

In `commands.rs`, after the `lore_sign_in` function:

```rust
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
```

In `lib.rs`, add to `generate_handler![...]` after `commands::lore_undo_commit,`:

```rust
        commands::lore_identity,
        commands::lore_sign_out,
```

- [x] **Step 2: Run Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS incl. `identity_tests`.

- [x] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(auth): lore_identity and real lore_sign_out commands"
```

---

### Task 2: Rust — status gains remote/revision fields

**Files:**
- Modify: `src-tauri/src/commands.rs:52-102` (`StatusResultDto`, `status_from`)

- [x] **Step 1: Extend the DTO + parser**

```rust
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
```

In `status_from`, after the `remote_ahead` computation:

```rust
    let revision_number = rev.and_then(|d| d.get("revisionNumber")).and_then(|v| v.as_u64()).unwrap_or(0);
    // Missing flags (older CLI) must not fake an outage — default to online.
    let remote_available = rev.and_then(|d| d.get("remoteAvailable")).map(json_truthy).unwrap_or(true);
    let remote_authorized = rev.and_then(|d| d.get("remoteAuthorized")).map(json_truthy).unwrap_or(true);
```

and include the three fields in the returned struct.

- [x] **Step 2: Fix compilation of existing tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Any test constructing `StatusResultDto` or asserting on `status_from` output needs the new fields (`revision_number: <from fixture>`, `remote_available: true`, `remote_authorized: true` for the existing fixtures). Add one assertion covering an offline fixture:

```rust
    #[test]
    fn parses_remote_flags() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionNumber":7,"revisionLocalNumber":7,"revisionRemoteNumber":7,"isLocalAhead":0,"isRemoteAhead":0,"remoteAvailable":0,"remoteAuthorized":1}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = crate::lore::parse_events(sample).unwrap();
        let s = status_from(&events);
        assert_eq!(s.revision_number, 7);
        assert!(!s.remote_available);
        assert!(s.remote_authorized);
    }
```

(Place it inside the existing status test module; if `status_from` has no test module yet, create `mod status_tests` with it. Note: Task 4 changes `status_from`'s signature — if executing tasks out of order, reconcile there.)

Expected: PASS.

- [x] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(status): expose revisionNumber and remote availability/authorization"
```

---

### Task 3: Rust — end of History pagination

**Files:**
- Modify: `src-tauri/src/commands.rs:295-325` (`lore_history`)

- [x] **Step 1: Add the pure rule + wire it**

Above `lore_history`:

```rust
/// End-of-history rule: a page shorter than requested means nothing older
/// remains, so pagination must stop (`None`) instead of re-serving the tail.
fn next_cursor_for(raw_len: usize, requested: u32, last_id: Option<String>) -> Option<String> {
    if (raw_len as u32) < requested { None } else { last_id }
}
```

In `lore_history`, right after `let mut page = history_from(&events);`:

```rust
        let raw_len = page.commits.len();
```

and after the existing cursor-dedup/labels block (end of the closure, before `Ok(page)`):

```rust
        page.next_cursor = next_cursor_for(raw_len, length, page.next_cursor.take());
```

- [x] **Step 2: Add tests**

```rust
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
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml` — expected PASS.

- [x] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "fix(history): stop pagination when a page comes back short"
```

---

### Task 4: Rust — binary detection (extended lists + NUL sniff)

**Files:**
- Modify: `src-tauri/src/commands.rs:61-64` (`BINARY_EXTS`/`is_binary_path`) and callers `status_from` (~line 91), `merge_conflicts_from` (~line 859), `lore_status`, `lore_merge_conflicts`, plus their tests

- [x] **Step 1: Replace the detector**

Replace `BINARY_EXTS` + `is_binary_path` with:

```rust
/// Known-binary game/DCC formats — fast path, no disk access.
const BINARY_EXTS: &[&str] = &[
    "uasset", "umap", "pak",
    "png", "tga", "dds", "exr", "hdr", "tif", "tiff", "jpg", "jpeg", "webp", "psd",
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

fn ext_of(path: &str) -> String {
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
```

- [x] **Step 2: Update the two parsers + their callers**

- `fn status_from(events: &[LoreEvent], repo_root: &std::path::Path) -> StatusResultDto` — inside, `is_binary: is_binary(repo_root, &path)`.
- `lore_status`: `Ok(status_from(&events, std::path::Path::new(&repo_path)))`.
- `fn merge_conflicts_from(events: &[LoreEvent], repo_root: &std::path::Path)` — same substitution; `lore_merge_conflicts` passes `std::path::Path::new(&repo_path)`.
- Update every existing test call site: pass `std::path::Path::new("")` (fixture paths don't exist on disk, so list-only decisions apply — existing fixtures use `.txt`/`.uasset`, still deterministic).

- [x] **Step 3: Add detector tests**

```rust
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
```

- [x] **Step 4: Run Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` — expected PASS (fix any remaining signature call sites it flags).

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(files): game-format extension lists plus NUL content sniff for binary detection"
```

---

### Task 5: Rust — `display_name` in the config

**Files:**
- Modify: `src-tauri/src/config.rs:7-14` (DTO) and `config.rs:67-79` (round-trip test)

- [x] **Step 1: Add the field**

```rust
#[derive(Serialize, Deserialize, Default, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigDto {
    pub server_url: Option<String>,
    pub current_repo: Option<String>,
    #[serde(default)]
    pub recent_repos: Vec<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}
```

In the `round_trip` test, add `display_name: Some("Jimmy D.".into()),` to the constructed config. The `missing_file_is_default` / `corrupt_file_is_default` tests need no change (`Default` covers the new field).

- [x] **Step 2: Run + commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` — expected PASS.

```bash
git add src-tauri/src/config.rs
git commit -m "feat(config): persist optional displayName"
```

---

### Task 6: TS — pure helpers (TDD) + types + mock/tauri parity

**Files:**
- Create: `src/lib/identity.ts`, `src/lib/identity.test.ts`, `src/lib/commitMessage.ts`, `src/lib/commitMessage.test.ts`
- Modify: `src/lib/types.ts`, `src/lib/mock.ts`, `src/lib/tauri.ts`

- [x] **Step 1: Write the failing tests**

`src/lib/identity.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { initialsFor, displayNameFor } from './identity'

describe('initialsFor', () => {
  it('uses the display name words', () => {
    expect(initialsFor('Jimmy D.', 'x@y.z')).toBe('JD')
  })
  it('takes two letters from a single-word display name', () => {
    expect(initialsFor('Zelda', null)).toBe('ZE')
  })
  it('falls back to the email local part, split on separators', () => {
    expect(initialsFor(null, 'jane.doe@studio.dev')).toBe('JD')
  })
  it('takes two letters from an unseparated email local part', () => {
    expect(initialsFor(null, 'jimmy@example.com')).toBe('JI')
  })
  it('is ? when nothing is known', () => {
    expect(initialsFor(null, null)).toBe('?')
    expect(initialsFor('  ', '')).toBe('?')
  })
})

describe('displayNameFor', () => {
  it('prefers the display name', () => {
    expect(displayNameFor('Jimmy D.', 'x@y.z')).toBe('Jimmy D.')
  })
  it('falls back to the email local part', () => {
    expect(displayNameFor(null, 'jane.doe@studio.dev')).toBe('jane.doe')
  })
  it('reports not signed in otherwise', () => {
    expect(displayNameFor(null, null)).toBe('Not signed in')
  })
})
```

`src/lib/commitMessage.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { composeCommitMessage } from './commitMessage'

describe('composeCommitMessage', () => {
  it('is the summary alone when the description is empty', () => {
    expect(composeCommitMessage('Fix hero mesh', '')).toBe('Fix hero mesh')
    expect(composeCommitMessage('Fix hero mesh', '   ')).toBe('Fix hero mesh')
  })
  it('joins summary and description with a blank line', () => {
    expect(composeCommitMessage(' Fix hero mesh ', ' Rebaked LODs. ')).toBe('Fix hero mesh\n\nRebaked LODs.')
  })
})
```

- [x] **Step 2: Run to verify failure**

Run: `npm test` — expected FAIL (modules missing).

- [x] **Step 3: Implement the helpers**

`src/lib/identity.ts`:

```ts
/** Pure display helpers for the signed-in identity (avatar initials, labels). */

function localPart(email: string | null | undefined): string {
  return email?.split('@')[0] ?? ''
}

/** Initials for the avatar: display name first, else the email local part. */
export function initialsFor(displayName: string | null | undefined, email: string | null | undefined): string {
  const src = displayName?.trim() || localPart(email)
  if (!src) return '?'
  const words = src.split(/[\s._-]+/).filter(Boolean)
  if (words.length >= 2) return (words[0][0] + words[1][0]).toUpperCase()
  return src.slice(0, 2).toUpperCase()
}

/** Human label for the identity: display name, else email local part. */
export function displayNameFor(displayName: string | null | undefined, email: string | null | undefined): string {
  return displayName?.trim() || localPart(email) || 'Not signed in'
}
```

`src/lib/commitMessage.ts`:

```ts
/** The stored commit message: summary, plus the description as body when present. */
export function composeCommitMessage(summary: string, description: string): string {
  const s = summary.trim()
  const d = description.trim()
  return d ? `${s}\n\n${d}` : s
}
```

- [x] **Step 4: Run to verify pass**

Run: `npm test` — expected PASS.

- [x] **Step 5: Extend the API surface (types + mock + tauri together, so `check` stays green)**

`src/lib/types.ts`:
- Add:

```ts
export interface Identity {
  id: string
  /** The account email as the server knows it (authUserInfo.name). */
  email: string
}
```

- `AppConfig`: add `displayName?: string | null`.
- `StatusResult`: add

```ts
  revisionNumber: number
  /** False when the server can't be reached (offline). */
  remoteAvailable: boolean
  /** False when the stored session is no longer accepted. */
  remoteAuthorized: boolean
```

- `LoreApi`: add

```ts
  /** Identity per the current repo's server; rejects when no repo/no session. */
  getIdentity(repoPath: string): Promise<Identity>
  /** Native file chooser starting inside the repo; absolute path or null if cancelled. */
  pickRepoFile(repoPath: string): Promise<string | null>
```

`src/lib/mock.ts`:
- `getStatus` return becomes:

```ts
    return {
      branch: s.branch, localAhead: s.localAhead, remoteAhead: s.remoteAhead,
      revisionNumber: 5, remoteAvailable: true, remoteAuthorized: true,
      files: [...s.files],
    } as StatusResult
```

- Add (near `pickFolder`):

```ts
  async getIdentity(_repoPath: string) {
    await delay(100)
    return { id: 'mock-user', email: 'jane.doe@studio.dev' }
  },
  async pickRepoFile(repoPath: string) {
    await delay(120)
    return `${repoPath}/Content/Environment/SM_Rock_02.uasset`
  },
```

- Import `Identity` type if needed by inference (usually not — literal suffices).

`src/lib/tauri.ts` — add overrides:

```ts
  signOut: () => invoke<void>('lore_sign_out'),
  getIdentity: (repoPath) => invoke<Identity>('lore_identity', { repoPath }),
  pickRepoFile: async (repoPath) => {
    const picked = await open({ directory: false, multiple: false, defaultPath: repoPath })
    return typeof picked === 'string' ? picked : null
  },
```

(and add `Identity` to the type import list).

- [x] **Step 6: Verify + commit**

Run: `npm run check && npm test` — expected PASS.

```bash
git add src/lib/identity.ts src/lib/identity.test.ts src/lib/commitMessage.ts src/lib/commitMessage.test.ts src/lib/types.ts src/lib/mock.ts src/lib/tauri.ts
git commit -m "feat(api): identity, repo file picker, remote flags - real sign-out wired"
```

---

### Task 7: Session — identity state + display name

**Files:**
- Modify: `src/lib/session.svelte.ts`, `src/App.svelte:37-40`

- [x] **Step 1: Session state + actions**

In `session.svelte.ts`:
- Import `Identity` from `./types`.
- Add `identity: null as Identity | null,` to the `$state` object.
- Add:

```ts
/** Fetch who we are on the current repo's server. Silent + best-effort: the
 *  offline indicator explains failures, the avatar just shows "?" meanwhile. */
export async function loadIdentity() {
  const path = session.config.currentRepo
  if (!path) { session.identity = null; return }
  try {
    session.identity = await api.getIdentity(path)
  } catch {
    session.identity = null
  }
}

export async function setDisplayName(name: string) {
  session.config = { ...session.config, displayName: name.trim() || null }
  await api.saveConfig(session.config)
}
```

- [x] **Step 2: Load on repo change**

In `App.svelte`, extend the existing repo-change effect:

```ts
  // Reload whenever the selected repository changes. refreshStatus also refreshes
  // locks + branches in the background, so they never block the initial render.
  $effect(() => {
    session.config.currentRepo
    refreshStatus()
    loadIdentity()
  })
```

(import `loadIdentity` from `./lib/session.svelte`).

- [x] **Step 3: Verify + commit**

Run: `npm run check && npm test` — expected PASS.

```bash
git add src/lib/session.svelte.ts src/App.svelte
git commit -m "feat(session): load real identity per repo, persist display name"
```

---

### Task 8: Changes — the Description finally commits

**Files:**
- Modify: `src/lib/Changes.svelte`

- [x] **Step 1: Bind + compose**

- Import: `import { composeCommitMessage } from './commitMessage'`
- Add state: `let description = $state('')`
- Textarea: `<textarea rows="2" placeholder="Description" bind:value={description} disabled={!!repo.busy}></textarea>`
- `doCommit`:

```ts
  async function doCommit() {
    const exclude = files.filter((f) => !staged.has(f.path)).map((f) => f.path)
    await commit(composeCommitMessage(message, description), exclude)
    message = ''
    description = ''
  }
```

- [x] **Step 2: Verify + commit**

Run: `npm run check && npm test` — expected PASS.

```bash
git add src/lib/Changes.svelte
git commit -m "fix(changes): commit the description instead of dropping it"
```

---

### Task 9: AvatarMenu + TitleBar wiring

**Files:**
- Create: `src/lib/AvatarMenu.svelte`
- Modify: `src/lib/TitleBar.svelte`

- [x] **Step 1: Create `src/lib/AvatarMenu.svelte`**

```svelte
<script lang="ts">
  import { session, signOut, setDisplayName } from './session.svelte'
  import { repo } from './repo.svelte'
  import { initialsFor, displayNameFor } from './identity'
  import Icon from './Icon.svelte'

  let { onclose }: { onclose: () => void } = $props()

  let name = $state(session.config.displayName ?? '')

  const email = $derived(session.identity?.email ?? null)
  const initials = $derived(initialsFor(session.config.displayName, email))
  const label = $derived(displayNameFor(session.config.displayName, email))

  async function saveName() {
    if ((session.config.displayName ?? '') === name.trim()) return
    await setDisplayName(name)
  }

  async function doSignOut() {
    await signOut()
    onclose()
  }
</script>

<div class="menu">
  <div class="who">
    <span class="ava">{initials}</span>
    <div class="ids">
      <span class="nm">{label}</span>
      <span class="em">{email ?? 'Open a repository to load your identity'}</span>
    </div>
  </div>
  <div class="field">
    <label for="dn">Display name</label>
    <input id="dn" bind:value={name} placeholder="e.g. Jimmy D." onblur={saveName}
           onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); saveName() } }} />
  </div>
  <div class="div"></div>
  <button class="action out" onclick={doSignOut} disabled={!!repo.busy}>
    <Icon name="external" size={15} /> Sign out
  </button>
</div>

<style>
  .menu { position: absolute; top: calc(100% + 6px); right: 0; width: 260px; background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); z-index: 50; overflow: hidden; padding: 8px 0; }
  .who { display: flex; align-items: center; gap: 10px; padding: 8px 14px 10px; }
  .ava { width: 34px; height: 34px; border-radius: 50%; background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; font-size: 12px; font-weight: 500; flex: none; }
  .ids { min-width: 0; display: flex; flex-direction: column; }
  .nm { font-size: 13px; font-weight: 500; }
  .em { font-size: 11.5px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .field { padding: 0 14px 10px; display: flex; flex-direction: column; gap: 4px; }
  .field label { font-size: 10.5px; color: var(--text-dim); }
  .field input { width: 100%; padding: 6px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .div { height: 1px; background: var(--border); margin: 2px 0 6px; }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 14px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover:not(:disabled) { background: var(--panel-hover); border: none; }
  .action.out { color: var(--deleted); }
  .action :global(svg) { color: currentColor; }
</style>
```

- [x] **Step 2: Wire the TitleBar avatar**

In `TitleBar.svelte`:
- Imports: replace `signOut` import with `session` only (`signOut` moves into the menu); add `import AvatarMenu from './AvatarMenu.svelte'` and `import { initialsFor } from './identity'`.
- Replace `const initials = 'JD'` with `const initials = $derived(initialsFor(session.config.displayName, session.identity?.email))`.
- Add state `let avatarOpen = $state(false)` and `let avatarZoneEl = $state<HTMLDivElement>()`, plus a third outside-click effect on the same pattern as `repoOpen`/`menuOpen`.
- Replace the avatar button with:

```svelte
  <div class="avatarzone" bind:this={avatarZoneEl}>
    <button class="avatar" class:open={avatarOpen} onclick={() => (avatarOpen = !avatarOpen)} title="Account">{initials}</button>
    {#if avatarOpen}<AvatarMenu onclose={() => (avatarOpen = false)} />{/if}
  </div>
```

- CSS: add `.avatarzone { position: relative; }` and `.avatar.open { outline: 2px solid var(--accent); }`.

- [x] **Step 3: Verify + commit**

Run: `npm run check && npm test` — expected PASS.

```bash
git add src/lib/AvatarMenu.svelte src/lib/TitleBar.svelte
git commit -m "feat(identity): avatar menu with real identity, editable display name, sign out"
```

---

### Task 10: History — 'you' mapping + drop dead row counters

**Files:**
- Modify: `src/lib/History.svelte`, `src/lib/types.ts:35-37`, `src/lib/mock.ts:39`

- [x] **Step 1: History edits**

- Delete row-counts span (`History.svelte:174`) — the whole `<span class="counts">…` line inside `.grow`. Keep the detail-panel counts (line 197) and the shared `.counts` CSS.
- Identity mapping — replace `avatar()` and `shortName`:

```ts
  const meEmail = $derived(session.identity?.email ?? null)
  const isMe = (name: string) => name === 'you' || (meEmail !== null && name === meEmail)

  function avatar(name: string) {
    const initials = isMe(name)
      ? initialsFor(session.config.displayName, meEmail)
      : name.split(/[\s._@-]+/).filter(Boolean).map((w) => w[0]).join('').slice(0, 2).toUpperCase() || '?'
    let h = 0; for (let i = 0; i < name.length; i++) h += name.charCodeAt(i)
    return { initials, ...PALETTE[h % PALETTE.length] }
  }
  // Compact author label: 'you' for the signed-in user, else the email local part.
  const shortName = (name: string) => (isMe(name) ? 'you' : name.includes('@') ? name.split('@')[0] : name)
```

(add `import { initialsFor } from './identity'`).

- [x] **Step 2: Retire the dead Commit fields**

- `types.ts`: delete `adds`, `mods`, `dels` from `Commit`.
- `mock.ts`: delete the `adds: …, mods: …, dels: …` line in `mk()` (`mock.ts:39`).
- Rust `CommitDto` keeps nothing to change (its `adds/mods/dels` fields must also go: remove them from the struct and from the `commits.push(CommitDto { … })` literal in `history_from`).

- [x] **Step 3: Verify + commit**

Run: `npm run check && npm test && cargo test --manifest-path src-tauri/Cargo.toml` — expected PASS.

```bash
git add src/lib/History.svelte src/lib/types.ts src/lib/mock.ts src-tauri/src/commands.rs
git commit -m "fix(history): real 'you' identity in rows, drop the never-populated row counters"
```

---

### Task 11: StatusBar states + offline-disabled actions

**Files:**
- Modify: `src/lib/StatusBar.svelte`, `src/lib/TitleBar.svelte`

- [x] **Step 1: StatusBar three states**

Replace the first `.item` block of `StatusBar.svelte` with:

```svelte
<script lang="ts">
  import { session, signOut } from './session.svelte'
  import { repo, locks } from './repo.svelte'
  import Icon from './Icon.svelte'

  const mine = $derived(locks.list.filter((l) => l.holder === 'you').length)
  const others = $derived(locks.list.filter((l) => l.holder !== 'you').length)
  const offline = $derived(repo.status ? !repo.status.remoteAvailable : false)
  const expired = $derived(repo.status ? repo.status.remoteAvailable && !repo.status.remoteAuthorized : false)
</script>

<footer class="statusbar">
  <span class="item">
    {#if repo.busy}
      <Icon name="sync" size={13} /> Working…
    {:else if !session.config.currentRepo}
      Not connected to a repository
    {:else if expired}
      <span class="dot bad"></span> <span class="bad">Session expired</span>
      <button class="mini" onclick={signOut}>Sign in again</button>
    {:else if offline}
      <span class="dot warn"></span> <span class="warn">Offline — changes stay local</span>
    {:else}
      <Icon name="check" size={13} /> Synced{#if repo.status?.revisionNumber}&nbsp;· rev {repo.status.revisionNumber}{/if}
    {/if}
  </span>
  <span class="spacer"></span>
  {#if session.config.currentRepo}
    <span class="item">
      <Icon name="lock" size={13} />
      {#if mine || others}
        {mine} held by you{#if others} · {others} by teammates{/if}
      {:else}
        No locks held
      {/if}
    </span>
  {/if}
</footer>
```

CSS additions:

```css
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .dot.warn { background: var(--modified); }
  .dot.bad { background: var(--deleted); }
  .warn { color: var(--modified); }
  .bad { color: var(--deleted); }
  .mini { margin-left: 6px; padding: 1px 8px; font-size: 11px; }
```

(`signOut` returns to the SignIn screen via `session.signedIn = false` — the existing App branch.)

- [x] **Step 2: Disable Sync/Push while unreachable**

In `TitleBar.svelte`:

```ts
  const noRemote = $derived(repo.status ? !repo.status.remoteAvailable || !repo.status.remoteAuthorized : false)
```

- Sync button: `disabled={!!repo.busy || noRemote}` and `title={noRemote ? 'Server unreachable — sync is unavailable' : 'Sync'}`.
- Push button: `disabled={!!repo.busy || noRemote || (repo.status?.localAhead ?? 0) === 0}` and `title={noRemote ? 'Server unreachable — push is unavailable' : 'Push'}`.

- [x] **Step 3: Verify + commit**

Run: `npm run check && npm test` — expected PASS.

```bash
git add src/lib/StatusBar.svelte src/lib/TitleBar.svelte
git commit -m "feat(status): offline and session-expired states, rev number, actions disabled offline"
```

---

### Task 12: Locks — working « + Lock a file… »

**Files:**
- Modify: `src/lib/Locks.svelte`

- [x] **Step 1: Wire the picker**

Script additions:

```ts
  import { api } from './api'
  import { toastError } from './toast'

  let locking = $state(false)

  /** Absolute picked path → repo-relative ('/'-separated), or null if outside the repo. */
  function toRepoRelative(absPath: string, repoRoot: string): string | null {
    const norm = (p: string) => p.replaceAll('\\', '/').replace(/\/+$/, '')
    const abs = norm(absPath)
    const root = norm(repoRoot)
    if (!abs.toLowerCase().startsWith(root.toLowerCase() + '/')) return null
    return abs.slice(root.length + 1)
  }

  async function lockNewFile() {
    const root = session.config.currentRepo
    if (!root || locking) return
    const picked = await api.pickRepoFile(root)
    if (!picked) return // cancelled
    const rel = toRepoRelative(picked, root)
    if (!rel) {
      toastError('Not in this repository', new Error(picked))
      return
    }
    locking = true
    try {
      await setLock(rel, true) // setLock toasts its own failures + refreshes
    } finally {
      locking = false
    }
  }
```

Button: `<button class="ghost" onclick={lockNewFile} disabled={locking || !!repo.busy}>{locking ? 'Locking…' : '+ Lock a file…'}</button>`

- [x] **Step 2: Verify + commit**

Run: `npm run check && npm test` — expected PASS.

```bash
git add src/lib/Locks.svelte
git commit -m "fix(locks): make 'Lock a file' pick and lock a repo file"
```

---

### Task 13: FilePreview — game-format type names

**Files:**
- Modify: `src/lib/FilePreview.svelte:46-49` (`TYPES`)

- [x] **Step 1: Extend the table**

```ts
  const TYPES: Record<string, string> = {
    uasset: 'Unreal asset', umap: 'Level (map)', pak: 'Unreal package',
    cpp: 'C++ source', h: 'C++ header', cs: 'C# source', ini: 'Config', md: 'Markdown', json: 'JSON',
    png: 'Texture', tga: 'Texture', dds: 'Texture', tif: 'Texture', tiff: 'Texture', jpg: 'Texture', jpeg: 'Texture', webp: 'Texture',
    exr: 'HDR texture', hdr: 'HDR texture', psd: 'Photoshop document',
    fbx: 'Mesh', obj: 'Mesh', abc: 'Alembic cache', gltf: 'Mesh', glb: 'Mesh',
    blend: 'Blender scene', ma: 'Maya scene', mb: 'Maya scene', max: '3ds Max scene', ztl: 'ZBrush tool',
    sbs: 'Substance graph', sbsar: 'Substance archive', spp: 'Substance Painter project',
    wav: 'Audio', ogg: 'Audio', mp3: 'Audio', flac: 'Audio', bank: 'Audio bank',
    anim: 'Animation',
  }
```

- [x] **Step 2: Verify + commit**

Run: `npm run check` — expected PASS.

```bash
git add src/lib/FilePreview.svelte
git commit -m "feat(preview): type names for common game and DCC formats"
```

---

### Task 14: Full verification

- [x] **Step 1: Full suites** — `npm run check && npm test && cargo test --manifest-path src-tauri/Cargo.toml` — all PASS.

- [x] **Step 2: Browser pass (mock)** — `npm run dev` + preview tools:
  1. Avatar shows « JD » (jane.doe) ; menu ouvre ; display name « Jimmy D. » → initiales changent, persistées après reload.
  2. Commit avec Summary + Description → mock accepte ; champs vidés.
  3. StatusBar montre « Synced · rev 5 ».
  4. Locks : « + Lock a file… » verrouille le fichier mock et il apparaît dans la liste.
  5. History : plus de compteurs sur les lignes ; auteurs 'you' avec les bonnes initiales.

- [x] **Step 3: Real app pass** — `npx tauri dev` sur `lore-test-repo` :
  1. Avatar = initiales dérivées de l'email réel ; menu affiche l'email.
  2. Commit avec description → `lore history` montre le corps du message.
  3. « + Lock a file… » verrouille un fichier réel (puis unlock).
  4. History scrolle jusqu'au bout sans requêtes en boucle.
  5. Sign out → retour à SignIn ; relance → toujours déconnecté (`lore auth list` vide d'identité valide).

- [x] **Step 4: Commit any fixes** discovered during verification.

---

## Self-review (planning time)

- **Spec coverage :** §1→Task 1+9, §2→Task 8 (+ helper Task 6), §3→Task 12 (FilePreview déjà couvert — déviation documentée en tête), §4→Task 3, §5→Task 10, §6→Tasks 1/5/6/7/9/10, §7→Tasks 2/11, §8→Tasks 4/13, parité mock→Tasks 6/10. ✔
- **Placeholders :** chaque étape code contient le code complet. ✔
- **Cohérence des types :** `Identity{id,email}` (Rust IdentityDto camelCase = id/email) ; `StatusResult.revisionNumber/remoteAvailable/remoteAuthorized` = DTO camelCase ; `getIdentity/pickRepoFile` identiques dans types/mock/tauri ; `initialsFor/displayNameFor/composeCommitMessage` mêmes signatures partout. ✔
