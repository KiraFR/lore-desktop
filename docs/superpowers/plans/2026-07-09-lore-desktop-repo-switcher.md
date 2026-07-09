# Persistent Repo Switcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-07-08-lore-desktop-repo-switcher-design.md`

**Goal:** Clicking **Current repository** opens a GitHub-Desktop-style dropdown listing known repos (persisted in `config.json` `recentRepos`) so the user can switch, clone, or add a repo without ever losing the current one.

**Architecture:** A new `RepoSwitcher.svelte` dropdown (modeled on `BranchMenu.svelte`) is toggled from the TitleBar repo button, replacing the current `clearCurrentRepo` behavior. Pure list logic lives in a new `repoList.ts` (unit-testable — the vitest config has no Svelte plugin, so runes files can't be imported in tests). The pick-folder/clone side effects are extracted from `RepoPicker.svelte` into a shared `repoActions.ts` used by both the first-run picker and the switcher.

**Tech Stack:** Svelte 5 (runes), TypeScript, Vitest (jsdom), Tauri 2 (unchanged).

**Commands used throughout** (run from repo root `lore-desktop/`):
- Tests: `npm test` (vitest run)
- Typecheck: `npm run check` (svelte-check + tsc)

---

## File structure

| File | Status | Responsibility |
|---|---|---|
| `src/lib/repoList.ts` | create | Pure helpers: MRU promote, remove, next-current fallback, filter, basename |
| `src/lib/repoList.test.ts` | create | Unit tests for the above |
| `src/lib/session.svelte.ts` | modify | `selectRepo` uses `promoteRepo` (no more 10-item cap), new `removeRepo`, bootstrap migration seeds `recentRepos` from `currentRepo`, `clearCurrentRepo` deleted (in Task 5) |
| `src/lib/repoActions.ts` | create | Shared side-effect helpers: `addExistingRepo()`, `cloneServerRepo(entry)` |
| `src/lib/RepoPicker.svelte` | modify | Delegates open/clone to `repoActions.ts` |
| `src/lib/RepoSwitcher.svelte` | create | The dropdown: filter, Add ▾ (Clone / Add existing), repo list, remove row action |
| `src/lib/TitleBar.svelte` | modify | Repo button toggles `RepoSwitcher` instead of calling `clearCurrentRepo` |
| `src/App.svelte` | unchanged | Full-screen `RepoPicker` still renders only when `currentRepo` is null |

---

### Task 1: Pure repo-list helpers (`repoList.ts`)

**Files:**
- Create: `src/lib/repoList.ts`
- Test: `src/lib/repoList.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/lib/repoList.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { repoName, promoteRepo, removeRepoPath, nextCurrentRepo, filterRepos } from './repoList'

describe('repoName', () => {
  it('returns the folder basename for Windows paths', () => {
    expect(repoName('C:\\SoonerOrLater\\game-main')).toBe('game-main')
  })
  it('returns the folder basename for POSIX paths', () => {
    expect(repoName('/home/jd/repos/audio')).toBe('audio')
  })
  it('ignores a trailing separator', () => {
    expect(repoName('C:/SoonerOrLater/game-main/')).toBe('game-main')
  })
})

describe('promoteRepo', () => {
  it('prepends a new path', () => {
    expect(promoteRepo(['a', 'b'], 'c')).toEqual(['c', 'a', 'b'])
  })
  it('moves an existing path to the front without duplicating it', () => {
    expect(promoteRepo(['a', 'b', 'c'], 'b')).toEqual(['b', 'a', 'c'])
  })
  it('keeps a front-most path in place', () => {
    expect(promoteRepo(['a', 'b'], 'a')).toEqual(['a', 'b'])
  })
})

describe('removeRepoPath', () => {
  it('drops the path', () => {
    expect(removeRepoPath(['a', 'b', 'c'], 'b')).toEqual(['a', 'c'])
  })
  it('is a no-op for an unknown path', () => {
    expect(removeRepoPath(['a'], 'zzz')).toEqual(['a'])
  })
})

describe('nextCurrentRepo', () => {
  it('keeps the current repo when another one was removed', () => {
    expect(nextCurrentRepo('a', 'b', ['a', 'c'])).toBe('a')
  })
  it('falls back to the most recent remaining repo when the current one was removed', () => {
    expect(nextCurrentRepo('a', 'a', ['b', 'c'])).toBe('b')
  })
  it('returns null when the last repo was removed', () => {
    expect(nextCurrentRepo('a', 'a', [])).toBeNull()
  })
})

describe('filterRepos', () => {
  const list = ['C:/SoonerOrLater/game-main', 'C:/SoonerOrLater/game-assets', 'D:/other/audio']
  it('returns everything for a blank query', () => {
    expect(filterRepos(list, '  ')).toEqual(list)
  })
  it('matches the repo name case-insensitively', () => {
    expect(filterRepos(list, 'AUDIO')).toEqual(['D:/other/audio'])
  })
  it('matches anywhere in the path', () => {
    expect(filterRepos(list, 'soonerorlater')).toEqual(['C:/SoonerOrLater/game-main', 'C:/SoonerOrLater/game-assets'])
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test`
Expected: FAIL — `Cannot find module './repoList'` (or equivalent resolve error).

- [ ] **Step 3: Write the implementation**

Create `src/lib/repoList.ts`:

```ts
/** Pure helpers for the known-repos list (`config.recentRepos`, MRU-first). */

/** Display name for a repo path: the folder basename. */
export function repoName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path
}

/** Move `path` to the front, prepending it if new — most-recently-used ordering. */
export function promoteRepo(list: string[], path: string): string[] {
  return [path, ...list.filter((r) => r !== path)]
}

/** Drop `path` from the list (does not touch files on disk). */
export function removeRepoPath(list: string[], path: string): string[] {
  return list.filter((r) => r !== path)
}

/** Current repo after removing `removed`: unchanged unless it *was* current, then the most recent remaining repo (or null). */
export function nextCurrentRepo(current: string | null, removed: string, remaining: string[]): string | null {
  return current === removed ? (remaining[0] ?? null) : current
}

/** Case-insensitive live filter over the full path (the name is part of the path). */
export function filterRepos(list: string[], query: string): string[] {
  const q = query.trim().toLowerCase()
  if (!q) return list
  return list.filter((p) => p.toLowerCase().includes(q))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test`
Expected: PASS — all `repoList` tests green, existing `toast`/`mock` tests still green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/repoList.ts src/lib/repoList.test.ts
git commit -m "feat(repos): pure helpers for the known-repos list"
```

---

### Task 2: Session state — promote/remove/migrate (`session.svelte.ts`)

**Files:**
- Modify: `src/lib/session.svelte.ts`

No unit tests here: the vitest config lacks the Svelte plugin, so `$state` runes files can't be imported in tests. All list logic is delegated to the Task 1 helpers, which are tested; verification is `npm run check`.

- [ ] **Step 1: Rewrite `selectRepo`, add `removeRepo`, migrate old configs in `bootstrap`**

Replace the entire content of `src/lib/session.svelte.ts` with:

```ts
import { api } from './api'
import { toastError } from './toast'
import { promoteRepo, removeRepoPath, nextCurrentRepo } from './repoList'
import type { AppConfig } from './types'

/** The studio's Lore server; used as the default when no server is stored yet. */
export const DEFAULT_SERVER_URL = 'lore://lore.example.com:41337'

// Shared reactive app state. `.svelte.ts` lets us use runes in a module.
export const session = $state({
  ready: false,
  signedIn: false,
  config: { serverUrl: null, currentRepo: null, recentRepos: [] } as AppConfig,
})

export async function bootstrap() {
  try {
    let config = await api.loadConfig()
    // A signed-in user shouldn't have to re-pick a server; default it when the
    // stored config has none so we go straight to the repo picker.
    if (!config.serverUrl) config = { ...config, serverUrl: DEFAULT_SERVER_URL }
    // Older configs set currentRepo without ever populating recentRepos; make
    // sure the open repo always appears in the known-repos list.
    if (config.currentRepo && !config.recentRepos.includes(config.currentRepo)) {
      config = { ...config, recentRepos: promoteRepo(config.recentRepos, config.currentRepo) }
    }
    session.config = config
    session.signedIn = await api.isAuthenticated()
  } catch (e) {
    toastError('Startup failed', e)
  } finally {
    session.ready = true
  }
}

export async function setSignedIn(serverUrl: string) {
  session.config = { ...session.config, serverUrl }
  await api.saveConfig(session.config)
  session.signedIn = true
}

/** Switch to (or add) a repo: set it current and move it to the front of the known list. */
export async function selectRepo(repoPath: string) {
  session.config = {
    ...session.config,
    currentRepo: repoPath,
    recentRepos: promoteRepo(session.config.recentRepos, repoPath),
  }
  await api.saveConfig(session.config)
}

/** Forget a repo (files stay on disk). If it was current, fall back to the next most recent. */
export async function removeRepo(repoPath: string) {
  const recent = removeRepoPath(session.config.recentRepos, repoPath)
  session.config = {
    ...session.config,
    currentRepo: nextCurrentRepo(session.config.currentRepo, repoPath, recent),
    recentRepos: recent,
  }
  await api.saveConfig(session.config)
}

export async function clearCurrentRepo() {
  session.config = { ...session.config, currentRepo: null }
  await api.saveConfig(session.config)
}

export async function signOut() {
  await api.signOut()
  session.signedIn = false
}
```

Notes:
- The old `selectRepo` capped the list at 10 (`.slice(0, 10)`); the cap is **deliberately dropped** — `recentRepos` is now the full known-repos list, per the spec.
- `clearCurrentRepo` is kept for now because `TitleBar.svelte` still imports it; it is deleted in Task 5.

- [ ] **Step 2: Typecheck and test**

Run: `npm run check && npm test`
Expected: both PASS (no component imports the new `removeRepo` yet — that's fine).

- [ ] **Step 3: Commit**

```bash
git add src/lib/session.svelte.ts
git commit -m "feat(repos): known-repos list in session state - promote, remove, config migration"
```

---

### Task 3: Shared open/clone actions (`repoActions.ts`) + RepoPicker refactor

**Files:**
- Create: `src/lib/repoActions.ts`
- Modify: `src/lib/RepoPicker.svelte`

These helpers wrap native dialogs and API calls (untestable side effects — toasts included); components keep only their busy-label state.

- [ ] **Step 1: Create `src/lib/repoActions.ts`**

```ts
import { api } from './api'
import { session, selectRepo } from './session.svelte'
import { toastError } from './toast'
import type { RepoEntry } from './types'

/**
 * Pick a local folder, validate it is a Lore working copy (has `.lore/`, which
 * `getStatus` checks), then add it to the known list and switch to it.
 * Returns true when the app switched repos.
 */
export async function addExistingRepo(): Promise<boolean> {
  const path = await api.pickFolder()
  if (!path) return false // cancelled
  try {
    await api.getStatus(path)
    await selectRepo(path)
    return true
  } catch (e) {
    toastError('Not a Lore repository', e)
    return false
  }
}

/**
 * Pick a destination parent folder, clone the server repo into it, then add it
 * to the known list and switch to it. Returns true when the app switched repos.
 */
export async function cloneServerRepo(entry: RepoEntry): Promise<boolean> {
  const parent = await api.pickFolder()
  if (!parent) return false // cancelled
  try {
    const path = await api.cloneRepo(session.config.serverUrl!, entry.id, entry.name, parent)
    await selectRepo(path)
    return true
  } catch (e) {
    toastError('Clone failed', e)
    return false
  }
}
```

- [ ] **Step 2: Refactor `RepoPicker.svelte` to use the helpers**

Replace the `<script>` block of `src/lib/RepoPicker.svelte` with (markup and styles unchanged):

```svelte
<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { addExistingRepo, cloneServerRepo } from './repoActions'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'
  import type { RepoEntry } from './types'

  let repos = $state<RepoEntry[]>([])
  let loading = $state(false)
  // '' | 'open' | `clone:<id>` — drives the in-flight button labels.
  let busy = $state('')

  async function loadRepos() {
    loading = true
    try {
      repos = await api.listRepos(session.config.serverUrl!)
    } catch (e) {
      toastError("Couldn't list repositories", e)
    } finally {
      loading = false
    }
  }

  async function openFolder() {
    busy = 'open'
    try {
      await addExistingRepo()
    } finally {
      busy = ''
    }
  }

  async function cloneRepo(entry: RepoEntry) {
    busy = `clone:${entry.id}`
    try {
      await cloneServerRepo(entry)
    } finally {
      busy = ''
    }
  }

  $effect(() => { loadRepos() })
</script>
```

(Behavior nuance: the busy label now also covers the native folder dialog, where before it only started after picking. Harmless — the window is blocked by the native dialog anyway.)

- [ ] **Step 3: Typecheck and test**

Run: `npm run check && npm test`
Expected: both PASS.

- [ ] **Step 4: Commit**

```bash
git add src/lib/repoActions.ts src/lib/RepoPicker.svelte
git commit -m "refactor(repos): extract open/clone actions shared by picker and switcher"
```

---

### Task 4: The dropdown (`RepoSwitcher.svelte`)

**Files:**
- Create: `src/lib/RepoSwitcher.svelte`

Modeled on `BranchMenu.svelte` (same popover shell, search input, `.sec` header, `.action` rows). Two modes: `list` (known repos + Add ▾) and `clone` (server repos). Not virtualized — a person's repo list stays small (spec: "Virtualized only if the list grows large (unlikely)").

Rows need a nested remove affordance, and buttons can't nest — so each row is a `.rowwrap` div holding the switch `<button class="item">` plus an absolutely-positioned `<button class="rm">` shown on hover.

- [ ] **Step 1: Create `src/lib/RepoSwitcher.svelte`**

```svelte
<script lang="ts">
  import { api } from './api'
  import { session, selectRepo, removeRepo } from './session.svelte'
  import { addExistingRepo, cloneServerRepo } from './repoActions'
  import { filterRepos, repoName } from './repoList'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'
  import type { RepoEntry } from './types'

  let { onclose }: { onclose: () => void } = $props()

  let filter = $state('')
  let addOpen = $state(false)
  // 'list' = known repos; 'clone' = pick a server repo to clone.
  let mode = $state<'list' | 'clone'>('list')
  let serverRepos = $state<RepoEntry[]>([])
  let loading = $state(false)
  // '' | 'add' | `clone:<id>` — drives the in-flight labels.
  let busy = $state('')

  const shown = $derived(filterRepos(session.config.recentRepos, filter))

  async function switchTo(path: string) {
    if (busy) return
    if (path !== session.config.currentRepo) await selectRepo(path)
    onclose()
  }

  async function onAddExisting() {
    addOpen = false
    if (busy) return
    busy = 'add'
    try {
      if (await addExistingRepo()) onclose()
    } finally {
      busy = ''
    }
  }

  async function enterClone() {
    addOpen = false
    mode = 'clone'
    loading = true
    try {
      serverRepos = await api.listRepos(session.config.serverUrl!)
    } catch (e) {
      toastError("Couldn't list repositories", e)
    } finally {
      loading = false
    }
  }

  async function onClone(entry: RepoEntry) {
    if (busy) return
    busy = `clone:${entry.id}`
    try {
      if (await cloneServerRepo(entry)) onclose()
    } finally {
      busy = ''
    }
  }
</script>

<div class="menu">
  {#if mode === 'list'}
    <div class="head">
      <input class="search" bind:value={filter} placeholder="Filter repositories" />
      <div class="addzone">
        <button class="add" class:open={addOpen} onclick={() => (addOpen = !addOpen)}>
          <Icon name="plus" size={13} /> Add <Icon name="chevronDown" size={12} />
        </button>
        {#if addOpen}
          <div class="addmenu">
            <button class="action" onclick={enterClone}><Icon name="sync" size={15} /> Clone repository…</button>
            <button class="action" onclick={onAddExisting} disabled={busy === 'add'}>
              <Icon name="folder" size={15} /> {busy === 'add' ? 'Opening…' : 'Add existing repository…'}
            </button>
          </div>
        {/if}
      </div>
    </div>
    <div class="sec">Repositories · {shown.length.toLocaleString()}</div>
    {#if shown.length === 0}
      <p class="empty">{filter.trim() ? 'No repositories match' : 'No repositories yet — use Add'}</p>
    {/if}
    <div class="list">
      {#each shown as path (path)}
        <div class="rowwrap">
          <button class="item" class:cur={path === session.config.currentRepo}
                  onclick={() => switchTo(path)} disabled={!!busy}>
            <Icon name="folder" size={15} />
            <span class="meta">
              <span class="rn">{repoName(path)}</span>
              <span class="rp">{path}</span>
            </span>
            {#if path === session.config.currentRepo}<Icon name="check" size={14} />{/if}
          </button>
          <button class="rm" title="Remove from list (files stay on disk)"
                  onclick={() => removeRepo(path)}>×</button>
        </div>
      {/each}
    </div>
  {:else}
    <div class="head">
      <button class="add" onclick={() => (mode = 'list')}><Icon name="chevronLeft" size={12} /> Back</button>
    </div>
    <div class="sec">Clone from {session.config.serverUrl}</div>
    {#if loading}<p class="empty">Loading repositories…</p>{/if}
    <div class="list">
      {#each serverRepos as r (r.id)}
        <div class="rowwrap">
          <button class="item" onclick={() => onClone(r)} disabled={!!busy}>
            <Icon name="folder" size={15} />
            <span class="meta">
              <span class="rn">{r.name}</span>
              <span class="rp">{busy === `clone:${r.id}` ? 'Cloning…' : r.id.slice(0, 12) + '…'}</span>
            </span>
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu { position: absolute; top: calc(100% + 6px); left: 0; width: 320px; background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); z-index: 50; overflow: hidden; padding: 8px 0; }
  .head { display: flex; align-items: center; gap: 6px; margin: 4px 10px 8px; }
  .search { flex: 1; min-width: 0; padding: 7px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .addzone { position: relative; }
  .add { display: flex; align-items: center; gap: 4px; padding: 6px 9px; font-size: 12px; }
  .add.open { background: var(--accent-soft); border-color: var(--accent); }
  .addmenu { position: absolute; top: calc(100% + 4px); right: 0; width: 230px; background: var(--panel); border: 1px solid var(--border-strong); border-radius: 8px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); z-index: 60; overflow: hidden; padding: 4px 0; }
  .sec { font-size: 10px; text-transform: uppercase; letter-spacing: .04em; color: var(--text-dim); padding: 2px 12px 5px; }
  .empty { margin: 2px 12px 8px; font-size: 12px; color: var(--text-muted); }
  .list { max-height: 300px; overflow-y: auto; overflow-x: hidden; }
  .rowwrap { position: relative; }
  .item { display: flex; align-items: center; gap: 9px; width: 100%; padding: 7px 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .item:hover:not(:disabled) { background: var(--panel-hover); border: none; }
  .item.cur { color: var(--accent-text); }
  .item :global(svg) { color: var(--text-muted); }
  .item.cur :global(svg) { color: var(--accent-text); }
  .meta { display: flex; flex-direction: column; min-width: 0; flex: 1; line-height: 1.25; }
  .rn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rp { font-size: 10.5px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; direction: rtl; text-align: left; }
  .rm { position: absolute; top: 50%; right: 8px; transform: translateY(-50%); display: none; width: 20px; height: 20px; padding: 0; line-height: 1; font-size: 14px; color: var(--text-muted); background: var(--panel); border: 1px solid var(--border); border-radius: 5px; }
  .rowwrap:hover .rm { display: block; }
  .rm:hover { color: var(--text); background: var(--panel-hover); }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover { background: var(--panel-hover); border: none; }
  .action :global(svg) { color: var(--text-muted); }
</style>
```

(The `.rp` path subtitle uses `direction: rtl` so long paths ellipsize at the *start*, keeping the informative tail — folder name end — visible. The check icon marks the current repo; the remove "×" only appears on row hover and sits above the row button, so a stray row click still switches while the × removes.)

- [ ] **Step 2: Typecheck**

Run: `npm run check`
Expected: PASS. (The component is not yet mounted anywhere — svelte-check still compiles it.)

- [ ] **Step 3: Commit**

```bash
git add src/lib/RepoSwitcher.svelte
git commit -m "feat(repos): RepoSwitcher dropdown - filter, add menu, clone panel, remove"
```

---

### Task 5: Wire the TitleBar, delete `clearCurrentRepo`

**Files:**
- Modify: `src/lib/TitleBar.svelte`
- Modify: `src/lib/session.svelte.ts:41-44` (delete `clearCurrentRepo`)

- [ ] **Step 1: Rewire the repo button in `TitleBar.svelte`**

Replace the entire content of `src/lib/TitleBar.svelte` with:

```svelte
<script lang="ts">
  import { session, signOut } from './session.svelte'
  import { repo, sync, push } from './repo.svelte'
  import Icon from './Icon.svelte'
  import BranchMenu from './BranchMenu.svelte'
  import RepoSwitcher from './RepoSwitcher.svelte'

  const repoName = $derived(session.config.currentRepo?.split(/[\\/]/).pop() || 'Select a repository')
  const initials = 'JD'
  let repoOpen = $state(false)
  let repoZoneEl = $state<HTMLDivElement>()
  let menuOpen = $state(false)
  let zoneEl = $state<HTMLDivElement>()

  // Close a menu when clicking anywhere outside its zone (button + popover).
  $effect(() => {
    if (!repoOpen) return
    function onDoc(e: PointerEvent) {
      if (repoZoneEl && !repoZoneEl.contains(e.target as Node)) repoOpen = false
    }
    document.addEventListener('pointerdown', onDoc)
    return () => document.removeEventListener('pointerdown', onDoc)
  })
  $effect(() => {
    if (!menuOpen) return
    function onDoc(e: PointerEvent) {
      if (zoneEl && !zoneEl.contains(e.target as Node)) menuOpen = false
    }
    document.addEventListener('pointerdown', onDoc)
    return () => document.removeEventListener('pointerdown', onDoc)
  })
</script>

<header class="titlebar">
  <div class="repozone" bind:this={repoZoneEl}>
    <button class="zone" class:open={repoOpen} onclick={() => (repoOpen = !repoOpen)} title="Switch repository">
      <Icon name="folder" size={16} />
      <div class="lbl"><span class="cap">Current repository</span><span class="val">{repoName}</span></div>
      <Icon name={repoOpen ? 'chevronUp' : 'chevronDown'} size={14} />
    </button>
    {#if repoOpen}<RepoSwitcher onclose={() => (repoOpen = false)} />{/if}
  </div>

  {#if session.config.currentRepo}
    <div class="branchzone" bind:this={zoneEl}>
      <button class="zone" class:open={menuOpen} onclick={() => (menuOpen = !menuOpen)} title="Current branch">
        <Icon name="branch" size={16} />
        <div class="lbl"><span class="cap">Current branch</span><span class="val">{repo.status?.branch ?? '…'}</span></div>
        <Icon name={menuOpen ? 'chevronUp' : 'chevronDown'} size={14} />
      </button>
      {#if menuOpen}<BranchMenu onclose={() => (menuOpen = false)} />{/if}
    </div>
  {/if}

  <span class="spacer"></span>

  {#if session.config.currentRepo}
    <button class="action" onclick={sync} disabled={!!repo.busy} title="Sync">
      <Icon name="sync" size={16} />
      <span>{repo.busy === 'sync' ? 'Syncing…' : 'Sync'}</span>
      {#if repo.status?.remoteAhead}<span class="count">{repo.status.remoteAhead}</span>{/if}
    </button>
    <button class="action accent" onclick={push} disabled={!!repo.busy || (repo.status?.localAhead ?? 0) === 0} title="Push">
      <Icon name="push" size={16} />
      <span>{repo.busy === 'push' ? 'Pushing…' : 'Push'}</span>
      {#if repo.status?.localAhead}<span class="count on">{repo.status.localAhead}</span>{/if}
    </button>
  {/if}

  <button class="avatar" onclick={signOut} title="Sign out">{initials}</button>
</header>

<style>
  .titlebar { display: flex; align-items: center; gap: 8px; height: 48px; padding: 0 10px; background: var(--bg-elev); border-bottom: 1px solid var(--border); position: relative; z-index: 20; }
  .zone { display: flex; align-items: center; gap: 8px; height: 34px; max-width: 220px; }
  .zone.open { background: var(--accent-soft); border-color: var(--accent); }
  .repozone { position: relative; }
  .branchzone { position: relative; }
  .lbl { display: flex; flex-direction: column; line-height: 1.15; min-width: 0; text-align: left; }
  .cap { font-size: 10.5px; color: var(--text-muted); }
  .val { font-size: 13px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .action { display: flex; align-items: center; gap: 6px; height: 32px; }
  .action .count { font-size: 11px; color: var(--text-muted); }
  .action .count.on { color: var(--on-accent); opacity: .85; }
  .avatar { width: 30px; height: 30px; border-radius: 50%; padding: 0; background: var(--accent-soft); color: var(--accent); border: none; font-size: 11px; font-weight: 500; }
</style>
```

- [ ] **Step 2: Delete `clearCurrentRepo` from `session.svelte.ts`**

Remove these lines (nothing imports it anymore):

```ts
export async function clearCurrentRepo() {
  session.config = { ...session.config, currentRepo: null }
  await api.saveConfig(session.config)
}
```

- [ ] **Step 3: Typecheck and test**

Run: `npm run check && npm test`
Expected: both PASS. If `check` reports a leftover `clearCurrentRepo` import, you missed the TitleBar import line.

- [ ] **Step 4: Commit**

```bash
git add src/lib/TitleBar.svelte src/lib/session.svelte.ts
git commit -m "feat(repos): TitleBar opens the repo switcher instead of dropping the repo"
```

---

### Task 6: End-to-end verification in the browser (mock API)

The vite dev server runs the app against the mock API (`api.ts` picks `mock` outside Tauri), which persists config to localStorage — the full switcher flow is exercisable without Tauri.

- [ ] **Step 1: Run the app** — `npm run dev`, open the served URL.

- [ ] **Step 2: Walk the flows** (sign in with the mock if needed):
  1. First run (clear localStorage): full-screen picker appears; clone `game-main` → app opens it.
  2. Click **Current repository** → dropdown opens, `game-main` listed + check mark; **the main view stays on the repo** (no full-screen picker).
  3. Add ▾ → *Clone repository…* → clone `audio` → app switches to it; dropdown reopened shows `audio` first (MRU), both repos listed.
  4. Add ▾ → *Add existing repository…* → mock returns `picked-repo` → switches.
  5. Filter field narrows the list.
  6. Hover a non-current row → × appears → removes it from the list only.
  7. Remove the *current* repo → app falls back to the next most recent repo.
  8. Reload the page → repo list and current repo persist (localStorage config).
  9. Remove **all** repos → full-screen picker returns (first-run state).

- [ ] **Step 3: Final full pass** — `npm run check && npm test`, then commit any fixes.

---

## Self-review (done at planning time)

- **Spec coverage:** persistence/MRU (Tasks 1–2), migration for never-populated `recentRepos` (Task 2), TitleBar no longer calls `clearCurrentRepo` (Task 5), switcher UI with filter + Add ▾ (Clone + Add existing only — **no Create**) + highlighted current row + remove action (Task 4), shared helpers extracted from RepoPicker (Task 3), first-run full-screen picker unchanged (`App.svelte` untouched), flat list / no grouping. ✔
- **Placeholder scan:** every code step contains the complete code. ✔
- **Type consistency:** `promoteRepo`/`removeRepoPath`/`nextCurrentRepo`/`filterRepos`/`repoName` (Task 1) match their uses in Tasks 2 and 4; `addExistingRepo`/`cloneServerRepo` (Task 3) match Task 4's imports; `removeRepo`/`selectRepo` signatures match. ✔
