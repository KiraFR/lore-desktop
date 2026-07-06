# Lore Desktop — Text Diffs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use `- [ ]`.

**Goal:** Show the real unified text diff (current revision vs working copy) for a selected text file in FilePreview, replacing the placeholder.

**Architecture:** A Rust `lore_diff` command shells `lore diff … --json`, parses the returned unified-diff patch into a structured `DiffLine[]`; `LoreApi.getDiff` calls it over `invoke`; `FilePreview.svelte` renders the lines colored by kind. Mock returns a canned diff.

**Tech Stack:** Tauri v2 (Rust, `run_lore`), Svelte 5 + TS, Vitest. No new deps/capabilities.

**Branch:** `wiring-textdiff`. Repo root: `C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop`. Commits: English, NO `Co-Authored-By: Claude` trailer.

---

## Task 1: Rust `lore_diff` + `parse_diff`

**Files:** Modify `src-tauri/src/commands.rs` (append after `lore_locks`, before/after existing test modules); Modify `src-tauri/src/lib.rs`.

- [ ] **Step 1: Write the failing test** — append to `commands.rs`:

```rust
#[cfg(test)]
mod diff_tests {
    use super::*;

    #[test]
    fn parses_unified_patch() {
        // Captured real `lore diff notes.txt --json` patch (CRLF line ends).
        let patch = "--- notes.txt@3\n+++ notes.txt\n@@ -1 +1,2 @@\n scratch notes\r\n+test notre\r\n";
        let lines = parse_diff(patch);
        assert_eq!(lines.len(), 3); // the --- / +++ headers are dropped
        assert_eq!(lines[0].kind, "hunk");
        assert_eq!(lines[1].kind, "context");
        assert_eq!(lines[1].text, " scratch notes"); // trailing \r stripped
        assert_eq!(lines[2].kind, "add");
        assert_eq!(lines[2].text, "+test notre");
    }
}
```

- [ ] **Step 2: Run — expect FAIL** (`parse_diff`/`DiffLineDto` undefined): `cargo test --manifest-path src-tauri/Cargo.toml diff_tests`

- [ ] **Step 3: Implement** — append to `commands.rs`:

```rust
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiffLineDto {
    pub kind: String, // "add" | "del" | "context" | "hunk"
    pub text: String,
}

/// Parse a unified-diff patch into structured lines. The `---`/`+++` file headers
/// are dropped; the `+`/`-`/space prefix is kept as a gutter.
fn parse_diff(patch: &str) -> Vec<DiffLineDto> {
    patch
        .lines()
        .filter_map(|raw| {
            let text = raw.trim_end_matches('\r').to_string();
            if text.starts_with("+++") || text.starts_with("---") {
                return None;
            }
            let kind = if text.starts_with("@@") {
                "hunk"
            } else if text.starts_with('+') {
                "add"
            } else if text.starts_with('-') {
                "del"
            } else {
                "context"
            };
            Some(DiffLineDto { kind: kind.to_string(), text })
        })
        .collect()
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
```

- [ ] **Step 4: Register** in `src-tauri/src/lib.rs` — add `commands::lore_diff,` to the `generate_handler![…]` list (after `commands::lore_locks,`).

- [ ] **Step 5: Run — expect PASS** + build: `cargo test --manifest-path src-tauri/Cargo.toml diff_tests` then `cargo build --manifest-path src-tauri/Cargo.toml`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(wiring): lore_diff command (unified text diff → DiffLine[])"
```

## Task 2: Frontend `LoreApi.getDiff` (types, mock, tauri)

**Files:** Modify `src/lib/types.ts`, `src/lib/mock.ts`, `src/lib/tauri.ts`, `src/lib/mock.test.ts`.

- [ ] **Step 1: Write the failing test** — in `src/lib/mock.test.ts`, add inside `describe('mock api', …)`:

```ts
  it('getDiff returns structured diff lines', async () => {
    const d = await mock.getDiff('C:/repos/game', 'src/x.ts')
    expect(d.length).toBeGreaterThan(0)
    expect(d.some((l) => l.kind === 'add')).toBe(true)
    expect(d[0]).toHaveProperty('text')
  })
```

- [ ] **Step 2: Run — expect FAIL** (`mock.getDiff` not a function): `npm test -- mock`

- [ ] **Step 3: Extend `LoreApi`** — in `src/lib/types.ts`, add the type (near the other interfaces, e.g. before `LoreApi`):

```ts
export interface DiffLine {
  kind: 'add' | 'del' | 'context' | 'hunk'
  text: string
}
```
and add to the `LoreApi` interface (e.g. after `getStatus`):
```ts
  getDiff(repoPath: string, path: string): Promise<DiffLine[]>
```

- [ ] **Step 4: Mock impl** — in `src/lib/mock.ts`, add to the `mock` object (after `getStatus`), and ensure `DiffLine` is imported in the top `import type { … } from './types'`:

```ts
  async getDiff(_repoPath: string, _path: string) {
    await delay(120)
    return [
      { kind: 'hunk', text: '@@ -1,3 +1,4 @@' },
      { kind: 'context', text: ' export const x = 1' },
      { kind: 'del', text: '-const y = 2' },
      { kind: 'add', text: '+const y = 3' },
      { kind: 'add', text: '+const z = 4' },
    ] as DiffLine[]
  },
```

- [ ] **Step 5: Tauri impl** — in `src/lib/tauri.ts`, add `DiffLine` to the `import type { … }` and add to the `tauriApi` object (after `getStatus`):

```ts
  getDiff: (repoPath, path) => invoke<DiffLine[]>('lore_diff', { repoPath, path }),
```

- [ ] **Step 6: Run — expect PASS** + typecheck: `npm test -- mock` then `npm run check`.

- [ ] **Step 7: Commit**

```bash
git add src/lib/types.ts src/lib/mock.ts src/lib/tauri.ts src/lib/mock.test.ts
git commit -m "feat(wiring): getDiff on LoreApi (mock + Tauri invoke)"
```

## Task 3: Render the diff in `FilePreview.svelte`

**Files:** Modify `src/lib/FilePreview.svelte`.

- [ ] **Step 1: Add imports + fetch state** — in the `<script>`, change the imports and add diff state + a fetch effect. Replace:

```ts
  import type { ChangedFile } from './types'
  import { repo, setLock } from './repo.svelte'
  import Icon from './Icon.svelte'

  let { file }: { file: ChangedFile | null } = $props()
```
with:
```ts
  import type { ChangedFile, DiffLine } from './types'
  import { repo, setLock } from './repo.svelte'
  import { session } from './session.svelte'
  import { api } from './api'
  import Icon from './Icon.svelte'

  let { file }: { file: ChangedFile | null } = $props()

  let diff = $state<DiffLine[]>([])
  let diffLoading = $state(false)
  let diffError = $state(false)

  // Fetch the unified diff whenever a text file is selected.
  $effect(() => {
    const f = file
    if (!f || f.isBinary) { diff = []; diffError = false; return }
    const repoPath = session.config.currentRepo
    if (!repoPath) { diff = []; return }
    diffLoading = true
    diffError = false
    api
      .getDiff(repoPath, f.path)
      .then((d) => { if (file?.path === f.path) diff = d })
      .catch(() => { if (file?.path === f.path) { diffError = true; diff = [] } })
      .finally(() => { if (file?.path === f.path) diffLoading = false })
  })
```

- [ ] **Step 2: Replace the text placeholder branch** — replace this block:

```svelte
      {:else}
        <div class="textnote muted">
          <Icon name="file" size={22} />
          <p>Text file — line-by-line diff arrives with real Lore wiring.</p>
        </div>
      {/if}
```
with:
```svelte
      {:else if diffLoading}
        <div class="textnote muted"><Icon name="file" size={22} /><p>Loading diff…</p></div>
      {:else if diffError}
        <div class="textnote muted"><Icon name="file" size={22} /><p>Couldn't load diff.</p></div>
      {:else if diff.length === 0}
        <div class="textnote muted"><Icon name="file" size={22} /><p>No text changes to show.</p></div>
      {:else}
        <div class="diff">
          {#each diff as line, i (i)}
            <div class="dl {line.kind}">{line.text}</div>
          {/each}
        </div>
      {/if}
```

- [ ] **Step 3: Add diff CSS** — inside the `<style>` block (e.g. after the `.textnote` rules), add:

```css
  .diff { font-family: var(--font-mono); font-size: 12px; line-height: 1.5; border: 1px solid var(--border); border-radius: 8px; overflow-x: auto; margin: 4px 0; }
  .dl { white-space: pre; padding: 0 10px; }
  .dl.add { background: rgba(63, 185, 80, .12); color: var(--added); }
  .dl.del { background: rgba(248, 81, 73, .12); color: var(--deleted); }
  .dl.context { color: var(--text-muted); }
  .dl.hunk { color: var(--accent-text); background: var(--panel); }
```

- [ ] **Step 4: Typecheck + tests** — `npm run check` (0/0) and `npm test` (all pass).

- [ ] **Step 5: Commit**

```bash
git add src/lib/FilePreview.svelte
git commit -m "feat(wiring): render unified text diff in FilePreview"
```

## Task 4: E2E + merge

- [ ] Merge `wiring-textdiff` → `main` + push.
- [ ] **E2E (user-assisted, `tauri dev`):** select a modified text file (e.g. `notes.txt`) in Changes → the preview shows the colored unified diff (context muted, `-` red, `+` green, `@@` accent) instead of the placeholder.
