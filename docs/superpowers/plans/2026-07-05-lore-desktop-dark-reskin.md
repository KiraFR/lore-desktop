# Lore Desktop — dark re-skin (Slice 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-skin the already-built Slice-1 Lore Desktop UI to the approved dark theme (GitHub-Desktop structure, studio dark skin) with a themeable CSS-token design system — no new features, mock behavior unchanged.

**Architecture:** All colors become CSS custom properties on `:root` (dark defaults) so a future theme is one token block plus a `data-theme` switch. Components reference tokens only. To match the mockup, the current repository's status (branch, ahead/behind) and the sync/push actions move from the Changes view up into the title bar; a tiny shared rune store (`repo.svelte.ts`) holds that status so both the title bar and the Changes view read the same source. A small `Icon.svelte` supplies the handful of line icons.

**Tech Stack:** Svelte 5 (runes) + Vite + TypeScript. Verification: `npm run check`, `npm test` (existing mock tests stay green), `npm run build`, plus a visual pass on `npm run dev`. Reference: `docs/superpowers/specs/2026-07-05-lore-desktop-visual-design.md`.

---

## Context the implementer needs

Existing files (built in the prior slice, on branch `slice-1-ui-mock`):
- `src/lib/api.ts` → `api` (`LoreApi`): `getStatus(repoPath)→StatusResult`, `commitAll(repoPath,message)`, `push(repoPath)`, `sync(repoPath)`, `listRepos`, `signIn`, `isAuthenticated`, `signOut`, `loadConfig`, `saveConfig`. Re-exports the types.
- `src/lib/types.ts` → `StatusResult {branch, localAhead, remoteAhead, files: ChangedFile[]}`, `ChangedFile {path, action:'add'|'modify'|'delete'|'move'|'copy', isBinary, size}`, `RepoEntry {id,name}`, `AppConfig`.
- `src/lib/session.svelte.ts` → reactive `session {ready, signedIn, config:{serverUrl,currentRepo,recentRepos}}` + `bootstrap`, `setSignedIn`, `selectRepo`, `clearCurrentRepo`, `signOut`.
- `src/App.svelte` (router), `src/lib/SignIn.svelte`, `src/lib/TitleBar.svelte`, `src/lib/RepoPicker.svelte`, `src/lib/Changes.svelte`, `src/lib/StatusPill.svelte`.
- `src/app.css` (current light-ish design system — to be replaced).
- Tests: `src/lib/mock.test.ts` (5 tests) — must stay green. No component tests exist; don't add a component-test framework.

Svelte 5 idioms in use: `$state`, `$props`, `$effect`, `$state`-modules named `*.svelte.ts`, `onclick={}`, `bind:value`, `{#each x as y (y.key)}`.

---

## File structure

- `src/app.css` — **rewrite**: themeable dark design-system tokens + global element styling.
- `src/lib/Icon.svelte` — **new**: single-`<path>` SVG line icon, `name` + `size` props.
- `src/lib/repo.svelte.ts` — **new**: shared rune store holding the current repo's `status` + `busy` + `error`, with `refreshStatus/commit/push/sync` actions bound to `session.config.currentRepo`.
- `src/lib/SignIn.svelte` — **rewrite markup/style**: standalone dark welcome (book icon, server field, Advanced, Sign in).
- `src/lib/TitleBar.svelte` — **rewrite**: repo · branch · Sync · Push · avatar; reads `repo` store; dark.
- `src/lib/Changes.svelte` — **rewrite**: Changes/History tab bar + file list + commit box; reads/acts via `repo` store; dark. (No sync/push here anymore — they moved to the title bar.)
- `src/lib/RepoPicker.svelte` — **restyle**: dark tokens.
- `src/lib/StatusBar.svelte` — **new**: bottom status bar (sync state + placeholder lock text).
- `src/App.svelte` — **modify**: mount `StatusBar` in the signed-in shell; trigger `refreshStatus()` when `currentRepo` changes.
- `src/lib/StatusPill.svelte` — **delete**: the ahead/behind counts now live on the Sync/Push buttons.

---

## Task 1: Themeable dark design system

**Files:** Modify `src/app.css`

- [ ] **Step 1: Rewrite `src/app.css`** entirely with:

```css
:root {
  --bg: #1b1d21;
  --bg-elev: #212429;
  --panel: #24272c;
  --panel-hover: #2a2e34;
  --border: #33373d;
  --border-strong: #3d424a;
  --text: #e6e8eb;
  --text-muted: #9198a1;
  --text-dim: #767c85;
  --accent: #3067d4;
  --accent-hover: #3a72e0;
  --accent-soft: #14304d;
  --accent-ring: rgba(48, 103, 212, 0.35);
  --on-accent: #ffffff;
  --added: #3fb950;
  --modified: #d29922;
  --deleted: #f85149;
  --warn-bg: #3d2f0f;
  --warn-text: #e3b341;
  --radius: 7px;
  --radius-lg: 10px;
  --font-sans: system-ui, -apple-system, "Segoe UI", sans-serif;
  --font-mono: ui-monospace, "Cascadia Code", monospace;
}

/* Add a theme later by overriding only the tokens it changes, e.g.
   :root[data-theme="light"] { --bg: #ffffff; --text: #1c1f23; ... }
   and setting document.documentElement.dataset.theme from saved config.
   No component changes needed — components read tokens only. */

* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; }
body { font-family: var(--font-sans); color: var(--text); background: var(--bg); font-size: 13.5px; }

button { font: inherit; cursor: pointer; border-radius: var(--radius); border: 1px solid var(--border); padding: 6px 12px; background: var(--panel); color: var(--text); transition: background .12s, border-color .12s; }
button:hover:not(:disabled) { background: var(--panel-hover); border-color: var(--border-strong); }
button:disabled { opacity: .5; cursor: default; }
button.accent { background: var(--accent); color: var(--on-accent); border-color: transparent; }
button.accent:hover:not(:disabled) { background: var(--accent-hover); }
button.ghost { background: transparent; border-color: transparent; color: var(--text-muted); }
button.ghost:hover:not(:disabled) { background: var(--panel); color: var(--text); }

input, textarea { font: inherit; width: 100%; padding: 8px 10px; border-radius: var(--radius); border: 1px solid var(--border-strong); background: var(--panel); color: var(--text); }
input::placeholder, textarea::placeholder { color: var(--text-dim); }
input:focus, textarea:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-ring); }

a { color: var(--accent); text-decoration: none; }
.muted { color: var(--text-muted); }
.dim { color: var(--text-dim); }
.error { color: var(--deleted); }
.spacer { flex: 1; }
```

- [ ] **Step 2: Type-check + build**

Run: `npm run check` — Expected: 0 errors (CSS change can't break types; components still reference old classes but that's cosmetic, not an error).
Run: `npm run build` — Expected: succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "feat(ui): themeable dark design-system tokens"
```

---

## Task 2: Icon component

**Files:** Create `src/lib/Icon.svelte`

- [ ] **Step 1: Create `src/lib/Icon.svelte`**

```svelte
<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props()

  // Feather-style 24x24 stroke icons; multiple subpaths share one <path d>.
  const paths: Record<string, string> = {
    book: 'M4 19.5A2.5 2.5 0 0 1 6.5 17H20 M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z',
    folder: 'M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z',
    branch: 'M6 3v12 M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6z M15 6a9 9 0 0 1-9 9',
    sync: 'M23 4v6h-6 M1 20v-6h6 M3.51 9a9 9 0 0 1 14.85-3.36L23 10 M1 14l4.64 4.36A9 9 0 0 0 20.49 15',
    push: 'M12 19V5 M5 12l7-7 7 7',
    external: 'M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6 M15 3h6v6 M10 14 21 3',
    chevronDown: 'M6 9l6 6 6-6',
    chevronRight: 'M9 6l6 6-6 6',
    check: 'M20 6 9 17l-5-5',
    lock: 'M19 11H5a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7a2 2 0 0 0-2-2z M7 11V7a5 5 0 0 1 10 0v4',
  }
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor"
     stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
     style="flex-shrink:0; display:block;">
  <path d={paths[name] ?? ''} />
</svg>
```

- [ ] **Step 2: Type-check**

Run: `npm run check` — Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/Icon.svelte
git commit -m "feat(ui): line-icon component"
```

---

## Task 3: Shared repo-status store + App wiring

**Files:** Create `src/lib/repo.svelte.ts`; Modify `src/App.svelte`

- [ ] **Step 1: Create `src/lib/repo.svelte.ts`**

```ts
import { api } from './api'
import { session } from './session.svelte'
import type { StatusResult } from './types'

// The current repository's status + in-flight action, shared by the title bar
// (branch, ahead/behind, sync, push) and the Changes view (files, commit).
export const repo = $state({
  status: null as StatusResult | null,
  busy: '' as '' | 'status' | 'commit' | 'push' | 'sync',
  error: '',
})

export async function refreshStatus() {
  const path = session.config.currentRepo
  if (!path) { repo.status = null; return }
  repo.error = ''; repo.busy = 'status'
  try { repo.status = await api.getStatus(path) }
  catch (e) { repo.error = String(e) }
  finally { repo.busy = '' }
}

async function act(kind: 'commit' | 'push' | 'sync', run: (path: string) => Promise<void>) {
  const path = session.config.currentRepo
  if (!path) return
  repo.error = ''; repo.busy = kind
  try { await run(path) }
  catch (e) { repo.error = String(e); repo.busy = ''; return }
  await refreshStatus()
}

export const commit = (message: string) => act('commit', (p) => api.commitAll(p, message))
export const push = () => act('push', (p) => api.push(p))
export const sync = () => act('sync', (p) => api.sync(p))
```

- [ ] **Step 2: Wire the refresh into `src/App.svelte`**

Replace `src/App.svelte` entirely with (adds the `refreshStatus` effect + a `StatusBar` in the signed-in shell; `StatusBar` is created in Task 7 — expect a transient missing-import error until then):

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { session, bootstrap } from './lib/session.svelte'
  import { refreshStatus } from './lib/repo.svelte'
  import SignIn from './lib/SignIn.svelte'
  import TitleBar from './lib/TitleBar.svelte'
  import RepoPicker from './lib/RepoPicker.svelte'
  import Changes from './lib/Changes.svelte'
  import StatusBar from './lib/StatusBar.svelte'

  onMount(bootstrap)

  // Reload status whenever the selected repository changes.
  $effect(() => {
    session.config.currentRepo
    refreshStatus()
  })
</script>

<main class="shell">
  {#if !session.ready}
    <div class="fill muted">Loading…</div>
  {:else if !session.signedIn}
    <SignIn />
  {:else}
    <TitleBar />
    <div class="body">
      {#if session.config.currentRepo}
        <div class="workarea">
          <Changes />
          <div class="preview">
            <div class="ph muted">
              <p>Select a file to preview.</p>
              <p class="dim small">Binary before/after compare arrives in a later update.</p>
            </div>
          </div>
        </div>
      {:else}
        <RepoPicker />
      {/if}
    </div>
    <StatusBar />
  {/if}
</main>

<style>
  .shell { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }
  .fill { display: grid; place-items: center; flex: 1; }
  .body { flex: 1; overflow: hidden; display: flex; }
  .workarea { flex: 1; display: flex; overflow: hidden; }
  .preview { flex: 1; display: grid; place-items: center; padding: 20px; }
  .ph { text-align: center; }
  .ph .small { font-size: 12px; margin-top: 4px; }
</style>
```

- [ ] **Step 3: Type-check**

Run: `npm run check` — Expected: errors ONLY about `./lib/StatusBar.svelte` (created in Task 7) and, because `Changes.svelte` still has a `repoPath` prop that `App.svelte` no longer passes, a Changes prop error. Both are expected and resolved in Tasks 6–7. No error should come from `repo.svelte.ts` itself.

- [ ] **Step 4: Commit**

```bash
git add src/lib/repo.svelte.ts src/App.svelte
git commit -m "feat(ui): shared repo-status store + refresh-on-repo-change wiring"
```

---

## Task 4: Sign-in — standalone dark welcome

**Files:** Modify `src/lib/SignIn.svelte`

- [ ] **Step 1: Replace `src/lib/SignIn.svelte`** entirely with:

```svelte
<script lang="ts">
  import { api } from './api'
  import { setSignedIn } from './session.svelte'
  import Icon from './Icon.svelte'

  let serverUrl = $state('lore://lore.example.com:41337')
  let authOverride = $state('')
  let showAdvanced = $state(false)
  let busy = $state(false)
  let error = $state('')

  async function go() {
    error = ''
    if (!serverUrl.startsWith('lore://') || serverUrl.length < 9) {
      error = 'Enter a Lore server URL like lore://host:41337'; return
    }
    busy = true
    try {
      await api.signIn(serverUrl.trim(), showAdvanced && authOverride.trim() ? authOverride.trim() : undefined)
      await setSignedIn(serverUrl.trim())
    } catch (e) { error = String(e) } finally { busy = false }
  }
</script>

<div class="signin">
  <div class="mark"><Icon name="book" size={26} /></div>
  <h1>Welcome to Lore Desktop</h1>
  <p class="muted sub">Connect to your Lore server to browse, commit, and push.</p>

  <div class="form">
    <label>Lore server</label>
    <input bind:value={serverUrl} placeholder="lore://host:41337" disabled={busy} />

    <button class="disclose" onclick={() => (showAdvanced = !showAdvanced)}>
      <Icon name={showAdvanced ? 'chevronDown' : 'chevronRight'} size={14} /> Advanced
    </button>
    {#if showAdvanced}
      <input bind:value={authOverride} placeholder="Auth service URL (optional)" disabled={busy} />
    {/if}

    <button class="accent go" onclick={go} disabled={busy}>
      <Icon name="external" size={16} />
      {busy ? 'Complete sign-in in your browser…' : 'Sign in'}
    </button>
    {#if error}<p class="error">{error}</p>{/if}
    <p class="dim hint">Signs in through your server's SSO · opens your browser</p>
  </div>
</div>

<style>
  .signin { max-width: 340px; margin: 12vh auto; padding: 0 20px; text-align: center; }
  .mark { width: 54px; height: 54px; border-radius: 50%; background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; margin: 0 auto 16px; }
  h1 { font-size: 20px; font-weight: 500; margin: 0 0 6px; }
  .sub { margin: 0 0 26px; font-size: 13px; }
  .form { text-align: left; }
  label { display: block; font-size: 12px; color: var(--text-muted); margin-bottom: 6px; }
  .disclose { display: inline-flex; align-items: center; gap: 4px; background: none; border: none; color: var(--accent); padding: 9px 0 0; }
  .disclose:hover { background: none; }
  .form input + .disclose { margin-top: 0; }
  .form input:nth-of-type(2) { margin-top: 8px; }
  .go { width: 100%; margin-top: 16px; padding: 10px; display: flex; align-items: center; justify-content: center; gap: 7px; font-weight: 500; }
  .hint { text-align: center; font-size: 12px; margin: 12px 0 0; }
  .error { text-align: center; margin: 10px 0 0; }
</style>
```

- [ ] **Step 2: Type-check**

Run: `npm run check` — Expected: the same expected StatusBar/Changes errors from Task 3 only; nothing new from `SignIn.svelte`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/SignIn.svelte
git commit -m "feat(ui): standalone dark sign-in (welcome + server field)"
```

---

## Task 5: Title bar — repo · branch · Sync · Push · avatar

**Files:** Modify `src/lib/TitleBar.svelte`

- [ ] **Step 1: Replace `src/lib/TitleBar.svelte`** entirely with:

```svelte
<script lang="ts">
  import { session, clearCurrentRepo, signOut } from './session.svelte'
  import { repo, sync, push } from './repo.svelte'
  import Icon from './Icon.svelte'

  const repoName = $derived(session.config.currentRepo?.split(/[\\/]/).pop() || 'Select a repository')
  const initials = 'JD'
</script>

<header class="titlebar">
  <button class="zone" onclick={clearCurrentRepo} title="Switch repository">
    <Icon name="folder" size={16} />
    <div class="lbl"><span class="cap">Current repository</span><span class="val">{repoName}</span></div>
    <Icon name="chevronDown" size={14} />
  </button>

  {#if session.config.currentRepo}
    <button class="zone" title="Current branch">
      <Icon name="branch" size={16} />
      <div class="lbl"><span class="cap">Current branch</span><span class="val">{repo.status?.branch ?? '…'}</span></div>
      <Icon name="chevronDown" size={14} />
    </button>
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
  .titlebar { display: flex; align-items: center; gap: 8px; height: 48px; padding: 0 10px; background: var(--bg-elev); border-bottom: 1px solid var(--border); }
  .zone { display: flex; align-items: center; gap: 8px; height: 34px; max-width: 210px; }
  .lbl { display: flex; flex-direction: column; line-height: 1.15; min-width: 0; text-align: left; }
  .cap { font-size: 10.5px; color: var(--text-muted); }
  .val { font-size: 13px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .action { display: flex; align-items: center; gap: 6px; height: 32px; }
  .action .count { font-size: 11px; color: var(--text-muted); }
  .action .count.on { color: var(--on-accent); opacity: .85; }
  .avatar { width: 30px; height: 30px; border-radius: 50%; padding: 0; background: var(--accent-soft); color: var(--accent); border: none; font-size: 11px; font-weight: 500; }
</style>
```

- [ ] **Step 2: Type-check**

Run: `npm run check` — Expected: only the pending StatusBar + Changes(`repoPath`) errors from Task 3; nothing new from `TitleBar.svelte`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/TitleBar.svelte
git commit -m "feat(ui): dark title bar with repo, branch, sync, push, avatar"
```

---

## Task 6: Changes view — dark tabs, file list, commit box

**Files:** Modify `src/lib/Changes.svelte`

- [ ] **Step 1: Replace `src/lib/Changes.svelte`** entirely with (note: no `repoPath` prop anymore — it reads the shared `repo` store):

```svelte
<script lang="ts">
  import { repo, commit } from './repo.svelte'

  let message = $state('')
  let tab = $state<'changes' | 'history'>('changes')

  const glyph: Record<string, { c: string; v: string }> = {
    add: { c: 'added', v: '+' }, modify: { c: 'modified', v: '~' }, delete: { c: 'deleted', v: '−' },
    move: { c: 'modified', v: 'R' }, copy: { c: 'modified', v: 'C' },
  }
  const dir = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }
  const base = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }

  const files = $derived(repo.status?.files ?? [])
  const branch = $derived(repo.status?.branch ?? 'main')

  async function doCommit() { await commit(message); message = '' }
</script>

<section class="changes">
  <div class="tabs">
    <button class="tab" class:active={tab === 'changes'} onclick={() => (tab = 'changes')}>Changes <span class="n">{files.length}</span></button>
    <button class="tab" class:active={tab === 'history'} onclick={() => (tab = 'history')}>History</button>
  </div>

  {#if repo.error}<p class="error pad">{repo.error}</p>{/if}

  {#if tab === 'history'}
    <div class="empty muted"><p>History arrives in a later update.</p></div>
  {:else}
    <div class="filelist">
      {#if repo.busy === 'status' && !repo.status}
        <p class="muted pad">Scanning…</p>
      {:else if files.length === 0}
        <div class="empty muted"><p>No local changes.</p></div>
      {:else}
        <ul>
          {#each files as f (f.path)}
            <li class="file">
              <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>
              <span class="path"><span class="dir">{dir(f.path)}</span>{base(f.path)}</span>
              {#if f.isBinary}<span class="bin">bin</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="composer">
      <input bind:value={message} placeholder="Summary (required)" disabled={!!repo.busy} />
      <textarea rows="2" placeholder="Description" disabled={!!repo.busy}></textarea>
      <button class="accent" onclick={doCommit} disabled={!!repo.busy || !message.trim() || files.length === 0}>
        {repo.busy === 'commit' ? 'Committing…' : `Commit to ${branch}`}
      </button>
    </div>
  {/if}
</section>

<style>
  .changes { display: flex; flex-direction: column; width: 340px; flex-shrink: 0; overflow: hidden; border-right: 1px solid var(--border); }
  .tabs { display: flex; border-bottom: 1px solid var(--border); }
  .tab { flex: 1; border: none; border-radius: 0; background: none; color: var(--text-muted); padding: 9px; font-size: 13px; }
  .tab:hover { background: var(--panel); color: var(--text); }
  .tab.active { color: var(--text); box-shadow: inset 0 -2px 0 var(--accent); }
  .tab .n { color: var(--text-dim); font-size: 12px; }
  .pad { padding: 8px 12px; }
  .filelist { flex: 1; overflow: auto; }
  .filelist ul { list-style: none; margin: 0; padding: 4px 0; }
  .file { display: flex; align-items: center; gap: 8px; padding: 5px 12px; font-size: 12.5px; }
  .file:hover { background: var(--panel); }
  .tag { width: 1.1em; text-align: center; font-weight: 500; }
  .tag.added { color: var(--added); } .tag.modified { color: var(--modified); } .tag.deleted { color: var(--deleted); }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dir { color: var(--text-muted); }
  .bin { font-size: 10px; padding: 1px 5px; border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted); }
  .empty { flex: 1; display: grid; place-items: center; }
  .composer { display: flex; flex-direction: column; gap: 8px; padding: 10px; border-top: 1px solid var(--border); background: var(--bg-elev); }
  .composer textarea { resize: none; }
</style>
```

- [ ] **Step 2: Type-check**

Run: `npm run check` — Expected: the `Changes` `repoPath` error is now gone; only the `StatusBar` missing-module error remains (fixed in Task 7).

- [ ] **Step 3: Commit**

```bash
git add src/lib/Changes.svelte
git commit -m "feat(ui): dark Changes view reading the shared repo store"
```

---

## Task 7: Repo picker restyle, status bar, and cleanup

**Files:** Modify `src/lib/RepoPicker.svelte`; Create `src/lib/StatusBar.svelte`; Delete `src/lib/StatusPill.svelte`

- [ ] **Step 1: Replace `src/lib/RepoPicker.svelte`** entirely with:

```svelte
<script lang="ts">
  import { api } from './api'
  import { session, selectRepo } from './session.svelte'
  import Icon from './Icon.svelte'
  import type { RepoEntry } from './types'

  let repos = $state<RepoEntry[]>([])
  let loading = $state(false)
  let error = $state('')

  async function loadRepos() {
    error = ''; loading = true
    try { repos = await api.listRepos(session.config.serverUrl!) }
    catch (e) { error = String(e) } finally { loading = false }
  }
  function fakeBrowse() { selectRepo('C:/SoonerOrLater/game-main') }

  $effect(() => { loadRepos() })
</script>

<div class="picker">
 <div class="inner">
  <h2>Open a repository</h2>

  <div class="card">
    <div><strong>Local working copy</strong><p class="muted small">Choose a folder you've already cloned.</p></div>
    <span class="spacer"></span>
    <button class="accent" onclick={fakeBrowse}>Open folder…</button>
  </div>

  <h3>On {session.config.serverUrl}</h3>
  {#if loading}<p class="muted">Loading repositories…</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  <ul class="repos">
    {#each repos as r (r.id)}
      <li>
        <span class="ico"><Icon name="folder" size={16} /></span>
        <div class="meta"><strong>{r.name}</strong><p class="muted small mono">{r.id.slice(0, 12)}…</p></div>
        <span class="spacer"></span>
        <button onclick={() => selectRepo(`C:/SoonerOrLater/${r.name}`)}>Clone…</button>
      </li>
    {/each}
  </ul>
 </div>
</div>

<style>
  .picker { flex: 1; overflow: auto; }
  .inner { max-width: 620px; margin: 6vh auto; padding: 0 20px; }
  h2 { font-size: 18px; font-weight: 500; margin: 0 0 16px; }
  h3 { margin: 22px 0 8px; font-size: 12px; color: var(--text-muted); font-weight: 500; }
  .card { display: flex; align-items: center; gap: 12px; border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 14px 16px; background: var(--panel); }
  .small { font-size: 12px; margin: 2px 0 0; }
  .mono { font-family: var(--font-mono); }
  .repos { list-style: none; padding: 0; margin: 0; }
  .repos li { display: flex; align-items: center; gap: 10px; padding: 10px 4px; border-bottom: 1px solid var(--border); }
  .ico { color: var(--accent); }
  .meta { min-width: 0; }
</style>
```

- [ ] **Step 2: Create `src/lib/StatusBar.svelte`**

```svelte
<script lang="ts">
  import { session } from './session.svelte'
  import { repo } from './repo.svelte'
  import Icon from './Icon.svelte'
</script>

<footer class="statusbar">
  <span class="item">
    {#if repo.busy}
      <Icon name="sync" size={13} /> Working…
    {:else if session.config.currentRepo}
      <Icon name="check" size={13} /> Synced
    {:else}
      Not connected to a repository
    {/if}
  </span>
  <span class="spacer"></span>
  {#if session.config.currentRepo}
    <span class="item"><Icon name="lock" size={13} /> No locks held</span>
  {/if}
</footer>

<style>
  .statusbar { display: flex; align-items: center; gap: 6px; height: 26px; padding: 0 12px; background: var(--bg-elev); border-top: 1px solid var(--border); font-size: 12px; color: var(--text-muted); }
  .item { display: inline-flex; align-items: center; gap: 5px; }
</style>
```

- [ ] **Step 3: Delete the now-unused StatusPill**

```bash
git rm src/lib/StatusPill.svelte
```

- [ ] **Step 4: Full type-check**

Run: `npm run check` — Expected: **0 errors** (all imports resolve; the app is whole and dark).

- [ ] **Step 5: Commit**

```bash
git add src/lib/RepoPicker.svelte src/lib/StatusBar.svelte
git commit -m "feat(ui): dark repo picker + bottom status bar; drop StatusPill"
```

---

## Task 8: Verify + visual pass

**Files:** none (verification only)

- [ ] **Step 1: Full verification**

Run: `npm run check` — Expected: 0 errors.
Run: `npm test` — Expected: the 5 mock tests still pass (no logic changed).
Run: `npm run build` — Expected: succeeds.

- [ ] **Step 2: Visual pass**

Run: `npm run dev`, open `http://localhost:5173`, and walk the flow: dark sign-in (book icon, server field) → Sign in → dark repo picker → open a repo → title bar shows repo · branch · Sync · Push · avatar, Changes view lists the files with colored +/~/− tags and `bin` chips, the commit box commits (list clears, Push count bumps in the title bar), Push zeroes it, the bottom status bar shows "Synced". Confirm everything is dark and uses the accent blue, with no leftover light surfaces.

> If a visual detail is off (spacing, a color), adjust the relevant component `<style>` or a token in `src/app.css` and re-check — tokens make global tweaks cheap.

- [ ] **Step 3: Commit any visual fixes**

```bash
git add -A
git commit -m "polish: dark re-skin visual pass"
```

(Skip if nothing changed.)

---

## Definition of done

- `npm run check` 0 errors, `npm test` 5 passing, `npm run build` succeeds.
- `npm run dev` shows the full flow in the approved dark theme: standalone sign-in, title bar (repo · branch · Sync · Push · avatar), dark Changes view, bottom status bar.
- Every color comes from a `:root` token in `src/app.css`; no component hardcodes a hex — a future theme is one token block plus a `data-theme` switch.
- No new features: still mock data, commit-all, no diff pane / locks / history / branches / merge (those are later slices).
