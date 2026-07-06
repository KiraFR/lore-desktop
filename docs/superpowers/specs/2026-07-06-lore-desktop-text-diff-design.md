# Lore Desktop — text diffs in FilePreview

- **Date:** 2026-07-06
- **Repo:** github.com/KiraFR/lore-desktop, branch `wiring-textdiff`
- **Ticket:** TICKET-127 (Lore Desktop)
- **Status:** design approved; next = writing-plans → inline implementation
- **Builds on:** Slices A–D (mock→real Tauri-command pipeline; `run_lore` NDJSON helper).

## Problem

`FilePreview.svelte` shows a placeholder for text files — "Text file — line-by-line diff arrives with real Lore wiring." The real diff is not wired.

## Goal

For a selected **text** file in the Changes view, show its real line-by-line **unified diff** (current revision vs the working-copy file) from `lore diff`, colored. Binary files keep the existing Before/After visual compare (unchanged).

## Architecture (Slice A pattern)

A Rust `lore_diff` command shells `lore diff … --json`, parses the returned unified-diff patch string into a structured `DiffLine[]`, and returns it. `LoreApi.getDiff` calls it over `invoke`; `FilePreview.svelte` renders the `DiffLine[]` for text files. The mock returns a canned diff for browser dev.

## Command / wire format

`lore diff <absPath> --repository <repoPath> --json` emits one `fileDiff` event then `complete`:
```
{"tagName":"fileDiff","data":{"path":"notes.txt","patch":"--- notes.txt@3\n+++ notes.txt\n@@ -1 +1,2 @@\n scratch notes\r\n+test notre\r\n","action":"keep"}}
{"tagName":"complete","data":{"status":0}}
```
`patch` is a standard unified diff (`--- `/`+++ ` file headers, `@@ … @@` hunk headers, ` `/`+`/`-` line prefixes; lines may carry a trailing `\r`). Default source = current revision, target = working-copy filesystem state — exactly the modified-file diff we want.

**Path resolution:** like `lore lock`, `lore diff` resolves a relative path against the process cwd, not `--repository` → pass an **absolute** path (`Path::new(repoPath).join(path)`).

## Rust design (`commands.rs`)

- DTO `DiffLineDto { kind: String, text: String }` (`#[serde(rename_all = "camelCase")]`); `kind` ∈ `"add" | "del" | "context" | "hunk"`.
- Pure `parse_diff(patch: &str) -> Vec<DiffLineDto>`: for each line (after `trim_end` to drop trailing `\r`): a line starting with `+++` or `---` is a file header → **skip**; `@@` → `hunk`; `+` → `add`; `-` → `del`; anything else (incl. leading space) → `context`. `text` keeps the raw prefixed line (the `+`/`-`/space acts as the gutter).
- `lore_diff(repo_path: String, path: String) -> Result<Vec<DiffLineDto>, String>`: build the absolute path; `run_lore(&["diff", &abs_path, "--repository", &repo_path])?`; take the first `fileDiff` event's `patch` string (empty `Vec` if there is none, e.g. a binary or no-diff file); return `parse_diff(patch)`.
- Registered in `lib.rs`.

## Frontend

- `types.ts`: `interface DiffLine { kind: 'add' | 'del' | 'context' | 'hunk'; text: string }`; add `getDiff(repoPath: string, path: string): Promise<DiffLine[]>` to `LoreApi`.
- `tauri.ts`: `getDiff: (repoPath, path) => invoke<DiffLine[]>('lore_diff', { repoPath, path })`.
- `mock.ts`: `getDiff` returns a small canned `DiffLine[]` (a hunk + a context + an add) so browser dev renders something.
- `FilePreview.svelte`: for a text file (`!file.isBinary`), when the selected file changes, fetch `api.getDiff(currentRepo, file.path)` into local state and render the lines in a monospace block, colored by `kind` — **add = green, del = red, context = muted, hunk = accent** — replacing the placeholder note. Keep the Type / Size / Lock rows above. A loading state while fetching; on failure an inline muted "Couldn't load diff" (no toast — the preview is passive, a per-select toast would spam). The **binary** branch (Before/After visual compare) is unchanged.

## Error handling

`getDiff` failure → inline muted "Couldn't load diff" in the preview pane (not a toast). An empty result (no `fileDiff`, e.g. a file with no textual diff) → a muted "No text changes to show".

## Testing

- **Rust:** `parse_diff` on the captured `notes.txt` patch → the `---`/`+++` headers are skipped; one `hunk` (`@@ -1 +1,2 @@`); one `context` (` scratch notes`); one `add` (`+test notre`); trailing `\r` stripped.
- **Frontend:** vitest for the mock `getDiff` shape.
- **E2E in `tauri dev`:** select the modified `notes.txt` in Changes → the preview shows the colored unified diff instead of the placeholder.

## Out of scope

Side-by-side diff; binary visual compare (kept as-is / deferred); syntax highlighting; word-level intra-line diffing; diffing arbitrary revision pairs (uses the default current-revision vs working-copy).
