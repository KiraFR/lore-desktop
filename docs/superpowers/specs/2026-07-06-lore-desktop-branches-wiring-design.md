# Lore Desktop — Slice E item 2: Real Branches Wiring

**Date:** 2026-07-06
**Status:** Approved for planning

## Goal

Wire the branch picker (`BranchMenu.svelte`) to real Lore: list, switch, and create
branches against the `lore` CLI instead of the in-memory mock. The component is already
fully built; this is a backend-wiring slice following the same pattern as slices B/C/D
(typed Rust `#[tauri::command]` shelling `lore … --json` → camelCase DTO → `tauri.ts`
override → components untouched).

## Scope

In scope:
- Three Rust commands: `lore_branches`, `lore_switch_branch`, `lore_create_branch`.
- `tauri.ts` overrides for `getBranches` / `switchBranch` / `createBranch`.
- Drop the mock-only `rev` field from the `Branch` type and its one UI use.
- Add error toasts to `BranchMenu.svelte` load/switch/create paths.

Out of scope (later slices): real graph lanes and per-commit file stats in `getHistory`;
merge; selective staging. No pagination — `lore branch list` returns every branch in one
NDJSON stream and the existing list virtualization renders it.

## CLI ground truth (captured on `lore-test-repo`)

`lore branch list --json` emits one `branchListEntry` per branch per location:

```json
{"tagName":"branchListEntry","data":{"location":"local","id":"e726318b…","name":"main","category":"","latest":"a3e42ae…","stack":[],"creator":"8c25b13e…","created":1783270929978,"isCurrent":true,"archived":false}}
```

Each branch appears once under `location:"local"` and once under `location:"remote"`.
The stream has no revision number — only `latest` (a tip hash). `isCurrent` is meaningful
on the local entries.

`lore branch create <name> --json` creates the branch from the current latest **and
auto-switches to it** (verified: after `branch create wiring-probe`, `main.isCurrent`
became `false`). No `--from`/base argument is used.

`lore branch switch <name> --json` switches to an existing branch.

## Architecture

### Rust — `src-tauri/src/commands.rs`

```rust
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BranchDto {
    pub name: String,
    pub current: bool,
}

/// Union of local + remote `branchListEntry` events, deduped by name.
/// `current` comes from the local entry's `isCurrent`; archived branches are skipped.
fn branches_from(events: &[LoreEvent]) -> Vec<BranchDto> { /* see plan */ }

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

#[tauri::command]
pub fn lore_create_branch(repo_path: String, name: String) -> Result<(), String> {
    run_lore(&["branch", "create", &name, "--repository", &repo_path])?;
    Ok(())
}
```

`branches_from` rules:
- Iterate `branchListEntry` events in stream order.
- Skip entries with `archived == true`.
- Key on `name`. First occurrence establishes the row and its display order.
- `current` is `true` if any entry for that name has `isCurrent == true` (only local
  entries carry it). Fold `isCurrent` across both locations so order-independence holds.
- Preserve first-seen order (local entries stream before remote, so order follows the
  local list, then any remote-only branches append in their stream order).

`run_lore` already appends `--json` and returns `Err` on a non-zero `complete.status`, so
switch/create surface CLI failures as an `Err(String)` that the frontend turns into a toast.

Register the three commands in `lib.rs`'s `invoke_handler` generate list.

### Frontend

`src/lib/types.ts` — `Branch` becomes `{ name: string; current: boolean }` (remove `rev`).

`src/lib/tauri.ts` — add overrides:
```ts
getBranches: (repoPath) => invoke<Branch[]>('lore_branches', { repoPath }),
switchBranch: (repoPath, name) => invoke<void>('lore_switch_branch', { repoPath, name }),
createBranch: (repoPath, name) => invoke<void>('lore_create_branch', { repoPath, name }),
```
`createBranch` drops the `basedOn` argument at the invoke boundary; the `LoreApi`
signature keeps `basedOn` so `BranchMenu`'s call site is unchanged.

`src/lib/BranchMenu.svelte`:
- Replace `{#if b.current}<Icon check/>{:else}<span class="rev">#{b.rev}</span>{/if}`
  with `{#if b.current}<Icon check/>{/if}` — current shows the check, others show nothing
  on the right (name + colour dot identify the branch).
- Wrap `load`, `switchTo`, and `create` bodies in `try/catch` → `toastError` with titles
  "Couldn't load branches" / "Switch failed" / "Create failed", clearing `busy` in
  `finally`. This matches the app-wide toast pattern; today these awaits are unguarded.

`src/lib/mock.ts` — remove `rev` from `buildBranches`/`createBranch` so the mock still
compiles against the trimmed `Branch` type and the shape test stays green.

## Data flow

`BranchMenu` opens → `getBranches` → real branch list rendered. Switch/create call the
matching command, then `refreshStatus()` (already in place) updates `repo.status.branch`,
so the TitleBar branch label and the History dashed-edge baseline both reflect the new
branch. On any failure, a red toast appears and the menu stays open.

## Testing

- **Rust unit test** for `branches_from`: feed a captured event slice with local+remote
  entries for `main` (current) plus a remote-only `feature/x` and an `archived:true`
  branch → assert two rows, `main.current == true`, archived excluded, order preserved.
- **vitest**: the existing `getBranches` mock shape test, updated to assert no `rev`.
- **E2E in `tauri dev`** on `lore-test-repo`: open the branch menu (real branches listed,
  including the leftover `wiring-probe`), create a branch (menu closes, TitleBar shows the
  new name), switch back to `main` (TitleBar updates). Clean up `wiring-probe` before the
  E2E so the list is tidy.

## Cleanup

The probe branch `wiring-probe` left on `lore-test-repo` (its archive returned "Not found")
is removed as the first build step so the real list starts clean.
