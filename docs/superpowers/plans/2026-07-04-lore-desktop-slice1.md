# Lore Desktop — Slice 1 (UI-first with mock data) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a fully navigable, well-designed Lore Desktop UI — sign-in → repository selection → a Changes view that lists changes and commits/pushes/syncs — driven entirely by an in-memory **mock** so we can iterate on the design in the browser (`npm run dev`) with zero backend. No Tauri commands and no `lore` CLI in this slice.

**Architecture:** Svelte 5 (runes) + Vite + TypeScript. A single `src/lib/api.ts` is the app's data boundary; in this slice it's a **stateful mock** (`src/lib/mock.ts`) that returns fake data with realistic latency and mutates in-memory state (committing clears files, pushing zeroes "ahead", etc.), persisting light config to `localStorage`. Crucially, `api.ts` exposes the **exact TypeScript contract** (`src/lib/types.ts`) the real Tauri backend will implement in a later slice — so wiring becomes a drop-in swap of `api.ts` internals, with components untouched.

**Tech Stack:** Svelte 5.56, Vite 8, TypeScript 6; `vitest` for the mock. Runs as a plain web app during design (`npm run dev`); the Tauri shell still launches it unchanged (`npm run tauri dev`) but no Rust commands are added this slice.

---

## Why UI-first + mock

- **Design iteration loop:** `npm run dev` serves the app in a browser with instant HMR — tweak layout/CSS and see it live, no Rust build, no AppLocker friction, no server/WireGuard needed.
- **Clean seam:** components only ever import from `./api` and `./types`. Today `api.ts` re-exports the mock; in the wiring slice it re-exports a Tauri-`invoke` implementation with identical signatures. The mock's shapes mirror the real `lore --json` output (captured live 2026-07-04), so the contract is already correct.
- **Rich states to design:** the mock ships enough fake data to exercise every screen state — signed-out, signing-in, repo list, empty repo, repo with mixed changes (add/modify/delete + a binary `.uasset`), ahead/behind, committing/pushing/syncing spinners, and errors.

---

## Data contract (mirrors the future real backend + real `lore --json`)

```ts
export interface ChangedFile {
  path: string
  action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
  isBinary: boolean
  size: number
}
export interface StatusResult {
  branch: string
  localAhead: number
  remoteAhead: number
  files: ChangedFile[]
}
export interface RepoEntry { id: string; name: string }
export interface AppConfig {
  serverUrl: string | null
  currentRepo: string | null   // local working-dir path
  recentRepos: string[]
}
```

> Note: the real `lore status --json` uses `action: "keep"` for a modification; the mock normalizes to `"modify"` for UI clarity. The wiring slice maps `keep→modify` in `api.ts`, keeping components stable.

---

## File structure

**Frontend** (`src/`):
- `src/lib/types.ts` — the data contract above (shared now + later).
- `src/lib/mock.ts` — stateful in-memory mock: fake repos, per-repo change sets, simulated latency, mutations; `localStorage` config. Unit-tested.
- `src/lib/api.ts` — the app's data boundary. This slice: `export * from './mock'`. Later: swapped to a Tauri-invoke implementation.
- `src/lib/session.svelte.ts` — a small Svelte 5 rune store holding `signedIn` + `config` + helpers (shared reactive app state).
- `src/App.svelte` — top-level state machine: signed-out → `SignIn`; signed-in → `TitleBar` + `RepoPicker`/`Changes` (rewritten from scaffold).
- `src/lib/SignIn.svelte`, `src/lib/TitleBar.svelte`, `src/lib/RepoPicker.svelte`, `src/lib/Changes.svelte`, `src/lib/StatusPill.svelte`.
- `src/app.css` — design system (CSS variables, light/dark, layout primitives). Rewritten.
- Delete `src/lib/Counter.svelte`, `src/assets/*` scaffold images once unused.

No `src-tauri/` changes in this slice (the existing shell still runs the frontend).

---

## Task 0: Clean the scaffold + confirm the dev loop

**Files:**
- Modify: `package.json` (add `test` script)
- Create: `vitest.config.ts`
- Delete: `src/lib/Counter.svelte`
- Modify: `src/App.svelte`, `src/app.css` (temporary minimal placeholder so the app builds clean)

- [ ] **Step 1: Add vitest**

Run (repo root):

```bash
npm install -D vitest@^3
```

Edit `package.json` `"scripts"` — add:

```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 2: vitest config**

Create `vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: { environment: 'jsdom', include: ['src/**/*.test.ts'] },
})
```

Install the jsdom env:

```bash
npm install -D jsdom@^25
```

- [ ] **Step 3: Replace App.svelte + app.css with a minimal clean placeholder**

Replace `src/App.svelte` with:

```svelte
<script lang="ts">
  // Placeholder — real UI arrives in later tasks.
</script>

<main class="app"><p class="muted">Lore Desktop</p></main>
```

Replace `src/app.css` with:

```css
:root { font-family: system-ui, -apple-system, sans-serif; }
* { box-sizing: border-box; }
body { margin: 0; }
.muted { opacity: 0.6; }
.app { display: grid; place-items: center; height: 100vh; }
```

- [ ] **Step 4: Delete the scaffold Counter**

```bash
git rm src/lib/Counter.svelte
```

- [ ] **Step 5: Verify the dev loop + type-check**

Run: `npm run check`
Expected: 0 errors.

Run: `npm run build`
Expected: builds successfully (vite outputs `dist/`).

> Manual (optional): `npm run dev` opens `http://localhost:5173` showing "Lore Desktop". This is the design-iteration server you'll use throughout.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: add vitest, strip scaffold demo, minimal app placeholder"
```

---

## Task 1: Data contract + stateful mock

**Files:**
- Create: `src/lib/types.ts`
- Create: `src/lib/mock.ts`
- Create: `src/lib/api.ts`
- Create: `src/lib/mock.test.ts`

- [ ] **Step 1: The types**

Create `src/lib/types.ts`:

```ts
export interface ChangedFile {
  path: string
  action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
  isBinary: boolean
  size: number
}

export interface StatusResult {
  branch: string
  localAhead: number
  remoteAhead: number
  files: ChangedFile[]
}

export interface RepoEntry {
  id: string
  name: string
}

export interface AppConfig {
  serverUrl: string | null
  currentRepo: string | null
  recentRepos: string[]
}

/** The data boundary the whole app uses. Mock now; Tauri-invoke later. */
export interface LoreApi {
  isAuthenticated(): Promise<boolean>
  signIn(serverUrl: string, authUrlOverride?: string): Promise<void>
  signOut(): Promise<void>
  listRepos(serverUrl: string): Promise<RepoEntry[]>
  getStatus(repoPath: string): Promise<StatusResult>
  commitAll(repoPath: string, message: string): Promise<void>
  push(repoPath: string): Promise<void>
  sync(repoPath: string): Promise<void>
  loadConfig(): Promise<AppConfig>
  saveConfig(config: AppConfig): Promise<void>
}
```

- [ ] **Step 2: Write the failing test**

Create `src/lib/mock.test.ts`:

```ts
import { describe, it, expect, beforeEach } from 'vitest'
import { mock } from './mock'

describe('mock api', () => {
  beforeEach(async () => {
    localStorage.clear()
    await mock.signOut()
  })

  it('starts signed out, then signs in', async () => {
    expect(await mock.isAuthenticated()).toBe(false)
    await mock.signIn('lore://demo:41337')
    expect(await mock.isAuthenticated()).toBe(true)
  })

  it('lists fake repos', async () => {
    const repos = await mock.listRepos('lore://demo:41337')
    expect(repos.length).toBeGreaterThan(0)
    expect(repos[0]).toHaveProperty('name')
  })

  it('getStatus returns a branch + files', async () => {
    const s = await mock.getStatus('C:/repos/game')
    expect(s.branch).toBe('main')
    expect(s.files.length).toBeGreaterThan(0)
    expect(s.files.some((f) => f.isBinary)).toBe(true)
  })

  it('commit clears files and bumps ahead; push zeroes ahead', async () => {
    const before = await mock.getStatus('C:/repos/game')
    expect(before.files.length).toBeGreaterThan(0)
    await mock.commitAll('C:/repos/game', 'my commit')
    const afterCommit = await mock.getStatus('C:/repos/game')
    expect(afterCommit.files.length).toBe(0)
    expect(afterCommit.localAhead).toBe(before.localAhead + 1)
    await mock.push('C:/repos/game')
    const afterPush = await mock.getStatus('C:/repos/game')
    expect(afterPush.localAhead).toBe(0)
  })

  it('persists config to localStorage', async () => {
    await mock.saveConfig({ serverUrl: 'lore://x:1', currentRepo: 'C:/r', recentRepos: ['C:/r'] })
    const cfg = await mock.loadConfig()
    expect(cfg.serverUrl).toBe('lore://x:1')
    expect(cfg.currentRepo).toBe('C:/r')
  })
})
```

- [ ] **Step 3: Run test to verify it fails**

Run: `npm test`
Expected: FAIL — `./mock` has no `mock` export.

- [ ] **Step 4: Implement the mock**

Create `src/lib/mock.ts`:

```ts
import type { AppConfig, ChangedFile, LoreApi, RepoEntry, StatusResult } from './types'

const delay = (ms = 350) => new Promise((r) => setTimeout(r, ms))
const CONFIG_KEY = 'loredesktop.config'
const AUTH_KEY = 'loredesktop.signedin'

const FAKE_REPOS: RepoEntry[] = [
  { id: '019f2e14006f7870a7b27df367c78b72', name: 'game-main' },
  { id: '019f2e1577257382bc89c5a28e3306cb', name: 'game-assets' },
  { id: '019f2e1699887744aa11bb22cc33dd44', name: 'audio' },
]

// Per-repo mutable change set, keyed by working-dir path. Defaults for any path.
function seedFiles(): ChangedFile[] {
  return [
    { path: 'Source/Player/PlayerCharacter.cpp', action: 'modify', isBinary: false, size: 8241 },
    { path: 'Source/Player/PlayerCharacter.h', action: 'modify', isBinary: false, size: 1204 },
    { path: 'Content/Characters/Hero/SK_Hero.uasset', action: 'add', isBinary: true, size: 4718592 },
    { path: 'Content/Maps/Level_01.umap', action: 'modify', isBinary: true, size: 2359296 },
    { path: 'Config/DefaultInput.ini', action: 'modify', isBinary: false, size: 512 },
    { path: 'Docs/old-notes.md', action: 'delete', isBinary: false, size: 0 },
  ]
}

interface RepoState { branch: string; localAhead: number; remoteAhead: number; files: ChangedFile[] }
const repoStates = new Map<string, RepoState>()

function stateFor(repoPath: string): RepoState {
  if (!repoStates.has(repoPath)) {
    repoStates.set(repoPath, { branch: 'main', localAhead: 0, remoteAhead: 1, files: seedFiles() })
  }
  return repoStates.get(repoPath)!
}

export const mock: LoreApi = {
  async isAuthenticated() {
    await delay(120)
    return localStorage.getItem(AUTH_KEY) === '1'
  },
  async signIn(_serverUrl: string) {
    await delay(700) // simulate the browser round-trip
    localStorage.setItem(AUTH_KEY, '1')
  },
  async signOut() {
    localStorage.removeItem(AUTH_KEY)
  },
  async listRepos(_serverUrl: string) {
    await delay()
    return FAKE_REPOS
  },
  async getStatus(repoPath: string) {
    await delay(250)
    const s = stateFor(repoPath)
    return { branch: s.branch, localAhead: s.localAhead, remoteAhead: s.remoteAhead, files: [...s.files] } as StatusResult
  },
  async commitAll(repoPath: string, message: string) {
    if (!message.trim()) throw new Error('commit message is required')
    await delay(500)
    const s = stateFor(repoPath)
    s.files = []
    s.localAhead += 1
  },
  async push(repoPath: string) {
    await delay(600)
    stateFor(repoPath).localAhead = 0
  },
  async sync(repoPath: string) {
    await delay(500)
    stateFor(repoPath).remoteAhead = 0
  },
  async loadConfig() {
    await delay(60)
    const raw = localStorage.getItem(CONFIG_KEY)
    if (raw) { try { return JSON.parse(raw) as AppConfig } catch { /* fall through */ } }
    return { serverUrl: null, currentRepo: null, recentRepos: [] }
  },
  async saveConfig(config: AppConfig) {
    localStorage.setItem(CONFIG_KEY, JSON.stringify(config))
  },
}
```

- [ ] **Step 5: Create the data boundary**

Create `src/lib/api.ts`:

```ts
// The app's single data boundary. In this slice it re-exports the mock; the
// wiring slice will replace the body with a Tauri-`invoke` implementation of the
// same `LoreApi` interface — components never change.
import { mock } from './mock'
import type { LoreApi } from './types'

export const api: LoreApi = mock
export * from './types'
```

- [ ] **Step 6: Run test to verify it passes**

Run: `npm test`
Expected: PASS (5 tests).

- [ ] **Step 7: Commit**

```bash
git add src/lib/types.ts src/lib/mock.ts src/lib/api.ts src/lib/mock.test.ts
git commit -m "feat(ui): data contract + stateful mock api (localStorage-backed)"
```

---

## Task 2: Session store + design system CSS

**Files:**
- Create: `src/lib/session.svelte.ts`
- Modify: `src/app.css` (design system)

- [ ] **Step 1: Reactive session store (Svelte 5 runes module)**

Create `src/lib/session.svelte.ts`:

```ts
import { api } from './api'
import type { AppConfig } from './types'

// Shared reactive app state. `.svelte.ts` lets us use runes in a module.
export const session = $state({
  ready: false,
  signedIn: false,
  config: { serverUrl: null, currentRepo: null, recentRepos: [] } as AppConfig,
})

export async function bootstrap() {
  session.config = await api.loadConfig()
  session.signedIn = await api.isAuthenticated()
  session.ready = true
}

export async function setSignedIn(serverUrl: string) {
  session.config = { ...session.config, serverUrl }
  await api.saveConfig(session.config)
  session.signedIn = true
}

export async function selectRepo(repoPath: string) {
  const recent = [repoPath, ...session.config.recentRepos.filter((r) => r !== repoPath)].slice(0, 10)
  session.config = { ...session.config, currentRepo: repoPath, recentRepos: recent }
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

- [ ] **Step 2: Design-system CSS**

Replace `src/app.css` with:

```css
:root {
  --bg: #ffffff; --bg-elev: #f6f7f9; --border: #e3e6ea;
  --text: #1c1f23; --muted: #6b7280; --accent: #3b82f6; --accent-text: #ffffff;
  --danger: #dc2626; --add: #16a34a; --modify: #d97706; --delete: #dc2626;
  --radius: 8px; --font: system-ui, -apple-system, "Segoe UI", sans-serif; --mono: ui-monospace, "Cascadia Code", monospace;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #16181d; --bg-elev: #1e2127; --border: #2b2f36;
    --text: #e6e8eb; --muted: #9aa2ad; --accent: #4f8ef7;
  }
}
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; }
body { font-family: var(--font); color: var(--text); background: var(--bg); font-size: 14px; }
button { font: inherit; cursor: pointer; border-radius: var(--radius); border: 1px solid var(--border); padding: 7px 12px; background: var(--bg-elev); color: var(--text); transition: filter .12s; }
button:hover:not(:disabled) { filter: brightness(1.05); }
button:disabled { opacity: .5; cursor: default; }
button.primary { background: var(--accent); color: var(--accent-text); border-color: transparent; }
button.ghost { background: transparent; border-color: transparent; color: var(--accent); padding: 4px 6px; }
input, textarea { font: inherit; width: 100%; padding: 9px 10px; border-radius: var(--radius); border: 1px solid var(--border); background: var(--bg); color: var(--text); }
input:focus, textarea:focus { outline: 2px solid var(--accent); outline-offset: -1px; }
.muted { color: var(--muted); }
.error { color: var(--danger); }
.row { display: flex; align-items: center; gap: 8px; }
.spacer { flex: 1; }
```

- [ ] **Step 3: Verify**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/session.svelte.ts src/app.css
git commit -m "feat(ui): reactive session store + design-system CSS (light/dark)"
```

---

## Task 3: App shell + Sign-in screen

**Files:**
- Create: `src/lib/SignIn.svelte`
- Modify: `src/App.svelte`

- [ ] **Step 1: Sign-in screen**

Create `src/lib/SignIn.svelte`:

```svelte
<script lang="ts">
  import { api } from './api'
  import { setSignedIn } from './session.svelte'

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
  <div class="logo">◆</div>
  <h1>Lore Desktop</h1>
  <p class="muted sub">Sign in to your Lore server</p>

  <label>Server URL
    <input bind:value={serverUrl} placeholder="lore://host:41337" disabled={busy} />
  </label>

  <button class="ghost adv" onclick={() => (showAdvanced = !showAdvanced)}>
    {showAdvanced ? '▾ Advanced' : '▸ Advanced'}
  </button>
  {#if showAdvanced}
    <label>Auth service URL (optional)
      <input bind:value={authOverride} placeholder="https://host:8081" disabled={busy} />
    </label>
  {/if}

  <button class="primary big" onclick={go} disabled={busy}>
    {busy ? 'Complete sign-in in your browser…' : 'Sign in'}
  </button>
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .signin { max-width: 360px; margin: 12vh auto; padding: 0 20px; text-align: center; }
  .logo { font-size: 44px; color: var(--accent); line-height: 1; }
  h1 { margin: 8px 0 2px; }
  .sub { margin-top: 0; }
  label { display: block; margin: 12px 0; text-align: left; font-size: 12px; color: var(--muted); }
  label input { margin-top: 4px; }
  .adv { display: block; margin: 0 auto 4px; }
  .big { width: 100%; padding: 11px; margin-top: 10px; }
</style>
```

- [ ] **Step 2: App shell / router**

Replace `src/App.svelte` with:

```svelte
<script lang="ts">
  import { onMount } from 'svelte'
  import { session, bootstrap } from './lib/session.svelte'
  import SignIn from './lib/SignIn.svelte'
  import TitleBar from './lib/TitleBar.svelte'
  import RepoPicker from './lib/RepoPicker.svelte'
  import Changes from './lib/Changes.svelte'

  onMount(bootstrap)
</script>

<main class="shell">
  {#if !session.ready}
    <div class="fill muted">Loading…</div>
  {:else if !session.signedIn}
    <SignIn />
  {:else}
    <TitleBar />
    {#if session.config.currentRepo}
      <Changes repoPath={session.config.currentRepo} />
    {:else}
      <RepoPicker />
    {/if}
  {/if}
</main>

<style>
  .shell { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }
  .fill { display: grid; place-items: center; flex: 1; }
</style>
```

- [ ] **Step 3: Type-check**

Run: `npm run check`
Expected: errors only about the not-yet-created `TitleBar`/`RepoPicker`/`Changes` imports. `SignIn.svelte` + its own logic type-clean.

- [ ] **Step 4: Commit**

```bash
git add src/App.svelte src/lib/SignIn.svelte
git commit -m "feat(ui): app shell router + sign-in screen"
```

---

## Task 4: Title bar + Repo picker

**Files:**
- Create: `src/lib/StatusPill.svelte`
- Create: `src/lib/TitleBar.svelte`
- Create: `src/lib/RepoPicker.svelte`

- [ ] **Step 1: Small ahead/behind pill**

Create `src/lib/StatusPill.svelte`:

```svelte
<script lang="ts">
  let { ahead = 0, behind = 0 }: { ahead?: number; behind?: number } = $props()
</script>

<span class="pill" title="{ahead} ahead / {behind} behind">
  <span class:dim={ahead === 0}>↑{ahead}</span>
  <span class:dim={behind === 0}>↓{behind}</span>
</span>

<style>
  .pill { display: inline-flex; gap: 6px; font-size: 12px; padding: 2px 8px; border: 1px solid var(--border); border-radius: 999px; }
  .dim { opacity: .4; }
</style>
```

- [ ] **Step 2: Title bar (repo · branch · sync · push · account)**

Create `src/lib/TitleBar.svelte`:

```svelte
<script lang="ts">
  import { session, clearCurrentRepo, signOut } from './session.svelte'

  function changeRepo() { clearCurrentRepo() }
</script>

<header class="titlebar">
  <button class="ghost repo" onclick={changeRepo} title="Switch repository">
    <span class="chip">◆</span>
    <span class="name">{session.config.currentRepo || 'Select a repository'}</span>
    <span class="caret">▾</span>
  </button>
  <span class="server muted">{session.config.serverUrl ?? ''}</span>
  <span class="spacer"></span>
  <button class="ghost" onclick={signOut} title="Sign out">Sign out</button>
</header>

<style>
  .titlebar { display: flex; align-items: center; gap: 10px; padding: 8px 12px; background: var(--bg-elev); border-bottom: 1px solid var(--border); }
  .repo { display: flex; align-items: center; gap: 8px; max-width: 50%; }
  .chip { color: var(--accent); }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; color: var(--text); }
  .server { font-size: 12px; }
</style>
```

- [ ] **Step 3: Repo picker (fake folder + server list)**

Create `src/lib/RepoPicker.svelte`:

```svelte
<script lang="ts">
  import { api } from './api'
  import { session, selectRepo } from './session.svelte'
  import type { RepoEntry } from './types'

  let repos = $state<RepoEntry[]>([])
  let loading = $state(false)
  let error = $state('')

  async function loadRepos() {
    error = ''; loading = true
    try { repos = await api.listRepos(session.config.serverUrl!) }
    catch (e) { error = String(e) } finally { loading = false }
  }

  // Fake "browse" — in the wiring slice this opens a native folder dialog.
  function fakeBrowse() { selectRepo('C:/SoonerOrLater/game-main') }

  $effect(() => { loadRepos() })
</script>

<div class="picker">
  <h2>Open a repository</h2>

  <div class="card">
    <div class="row">
      <div>
        <strong>Local working copy</strong>
        <p class="muted small">Choose a folder you've already cloned.</p>
      </div>
      <span class="spacer"></span>
      <button class="primary" onclick={fakeBrowse}>Open folder…</button>
    </div>
  </div>

  <h3>On {session.config.serverUrl}</h3>
  {#if loading}<p class="muted">Loading repositories…</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  <ul class="repos">
    {#each repos as r (r.id)}
      <li class="row">
        <span class="chip">◆</span>
        <div>
          <strong>{r.name}</strong>
          <p class="muted small mono">{r.id.slice(0, 12)}…</p>
        </div>
        <span class="spacer"></span>
        <button onclick={() => selectRepo(`C:/SoonerOrLater/${r.name}`)}>Clone…</button>
      </li>
    {/each}
  </ul>
</div>

<style>
  .picker { max-width: 620px; margin: 6vh auto; padding: 0 20px; overflow: auto; }
  h2 { margin-bottom: 16px; }
  h3 { margin: 22px 0 8px; font-size: 13px; color: var(--muted); font-weight: 600; }
  .card { border: 1px solid var(--border); border-radius: var(--radius); padding: 14px 16px; background: var(--bg-elev); }
  .small { font-size: 12px; margin: 2px 0 0; }
  .mono { font-family: var(--mono); }
  .repos { list-style: none; padding: 0; margin: 0; }
  .repos li { padding: 10px 4px; border-bottom: 1px solid var(--border); }
  .chip { color: var(--accent); }
</style>
```

- [ ] **Step 4: Type-check**

Run: `npm run check`
Expected: only the `Changes` import in `App.svelte` errors (created next task).

- [ ] **Step 5: Commit**

```bash
git add src/lib/StatusPill.svelte src/lib/TitleBar.svelte src/lib/RepoPicker.svelte
git commit -m "feat(ui): title bar + repository picker"
```

---

## Task 5: Changes view (the main screen)

**Files:**
- Create: `src/lib/Changes.svelte`

- [ ] **Step 1: Changes view**

Create `src/lib/Changes.svelte`:

```svelte
<script lang="ts">
  import { api } from './api'
  import StatusPill from './StatusPill.svelte'
  import type { StatusResult } from './types'

  let { repoPath }: { repoPath: string } = $props()

  let status = $state<StatusResult | null>(null)
  let message = $state('')
  let busy = $state<'' | 'status' | 'commit' | 'push' | 'sync'>('')
  let error = $state('')

  const tag: Record<string, { c: string; v: string }> = {
    add: { c: 'add', v: 'A' }, modify: { c: 'modify', v: 'M' }, delete: { c: 'delete', v: 'D' },
    move: { c: 'modify', v: 'R' }, copy: { c: 'modify', v: 'C' },
  }
  const kb = (n: number) => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.ceil(n / 1024)} KB`)

  async function refresh() {
    error = ''; busy = 'status'
    try { status = await api.getStatus(repoPath) }
    catch (e) { error = String(e) } finally { busy = '' }
  }
  async function run(kind: 'commit' | 'push' | 'sync') {
    error = ''; busy = kind
    try {
      if (kind === 'commit') { await api.commitAll(repoPath, message); message = '' }
      else if (kind === 'push') await api.push(repoPath)
      else await api.sync(repoPath)
      await refresh()
    } catch (e) { error = String(e) } finally { busy = '' }
  }

  $effect(() => { repoPath; refresh() })
</script>

<section class="changes">
  <div class="toolbar">
    <span class="branch">⎇ {status?.branch ?? '…'}</span>
    <StatusPill ahead={status?.localAhead ?? 0} behind={status?.remoteAhead ?? 0} />
    <span class="spacer"></span>
    <button onclick={refresh} disabled={!!busy}>↻</button>
    <button onclick={() => run('sync')} disabled={!!busy}>{busy === 'sync' ? 'Syncing…' : 'Sync'}</button>
    <button onclick={() => run('push')} disabled={!!busy || (status?.localAhead ?? 0) === 0}>
      {busy === 'push' ? 'Pushing…' : `Push${status?.localAhead ? ` (${status.localAhead})` : ''}`}
    </button>
  </div>

  {#if error}<p class="error pad">{error}</p>{/if}

  <div class="filelist">
    {#if busy === 'status' && !status}
      <p class="muted pad">Scanning…</p>
    {:else if status && status.files.length === 0}
      <div class="empty muted">
        <div class="big">✓</div>
        <p>No local changes.</p>
      </div>
    {:else}
      <ul>
        {#each status?.files ?? [] as f (f.path)}
          <li class="file">
            <span class="tag {tag[f.action]?.c}">{tag[f.action]?.v ?? '?'}</span>
            <span class="path">{f.path}</span>
            {#if f.isBinary}<span class="binary" title="Binary file">bin</span>{/if}
            <span class="spacer"></span>
            <span class="size muted">{f.action === 'delete' ? '—' : kb(f.size)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="composer">
    <textarea bind:value={message} rows="2" placeholder="Summary of your changes" disabled={!!busy}></textarea>
    <button class="primary" onclick={() => run('commit')}
      disabled={!!busy || !message.trim() || !status?.files.length}>
      {busy === 'commit' ? 'Committing…' : `Commit ${status?.files.length ?? 0} file${(status?.files.length ?? 0) === 1 ? '' : 's'}`}
    </button>
  </div>
</section>

<style>
  .changes { display: flex; flex-direction: column; flex: 1; overflow: hidden; }
  .toolbar { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-bottom: 1px solid var(--border); }
  .branch { font-weight: 600; }
  .pad { padding: 8px 12px; }
  .filelist { flex: 1; overflow: auto; }
  .filelist ul { list-style: none; margin: 0; padding: 4px 0; }
  .file { display: flex; align-items: center; gap: 10px; padding: 5px 14px; font-family: var(--mono); font-size: 12.5px; }
  .file:hover { background: var(--bg-elev); }
  .tag { width: 1.4em; text-align: center; font-weight: 700; border-radius: 4px; }
  .tag.add { color: var(--add); } .tag.modify { color: var(--modify); } .tag.delete { color: var(--delete); }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .binary { font-size: 10px; padding: 0 5px; border: 1px solid var(--border); border-radius: 999px; color: var(--muted); }
  .size { font-size: 11px; }
  .empty { display: grid; place-items: center; height: 100%; gap: 4px; }
  .empty .big { font-size: 40px; color: var(--add); }
  .composer { display: flex; flex-direction: column; gap: 8px; padding: 12px; border-top: 1px solid var(--border); background: var(--bg-elev); }
  .composer textarea { resize: none; }
</style>
```

- [ ] **Step 2: Type-check the whole app**

Run: `npm run check`
Expected: PASS (0 errors).

- [ ] **Step 3: Run the mock tests**

Run: `npm test`
Expected: PASS (Task 1's 5 tests).

- [ ] **Step 4: Build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/lib/Changes.svelte
git commit -m "feat(ui): Changes view — file list, commit composer, push/sync"
```

---

## Task 6: Design pass + README

**Files:**
- Modify: any `.svelte` / `src/app.css` per visual review
- Modify: `README.md`

> This task is the design-iteration surface. Run `npm run dev` and walk the flow: sign-in → repo picker → Changes (with mixed files + a binary) → commit (list clears, ahead bumps) → push (ahead zeroes) → sync. Tune spacing, colors, typography, empty/loading/busy states, and dark mode until it feels like a polished GitHub-Desktop-class client.

- [ ] **Step 1: Visual walkthrough + polish**

Run: `npm run dev`, exercise every state, and refine CSS/markup. Keep changes within the existing components + `app.css` (the design system variables make global tweaks cheap). No new data or backend.

- [ ] **Step 2: README**

Replace `README.md` with a short overview: what Slice 1 is (a mock-data UI for design iteration), how to run it (`npm install`, `npm run dev` for design, `npm test`, `npm run check`), the `src/lib/api.ts` seam (mock now → Tauri invoke later), and a "next slice" note (wire the real `lore` CLI backend behind the same `LoreApi` interface).

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "polish: design pass on Slice 1 UI + README"
```

---

## Definition of done

- `npm run check` — 0 type errors.
- `npm test` — mock tests pass.
- `npm run build` — succeeds.
- `npm run dev` — the full flow is navigable and designed with mock data: sign-in → repo picker → Changes → commit → push → sync, including empty/loading/busy/error states and dark mode.
- The app touches **no** backend: every data call goes through `src/lib/api.ts` (mock), whose `LoreApi` interface matches the real `lore --json` shapes for a clean wiring-slice swap.

## Deferred to the wiring slice (not this plan)

- Rust: `src-tauri` commands shelling the `lore` CLI + NDJSON parsers (the shapes are captured in git history of this plan / memory).
- Replace `src/lib/api.ts` internals with `@tauri-apps/api` `invoke` calls (same `LoreApi` signatures), add `@tauri-apps/plugin-dialog` for the real folder picker, map `lore status` `action:"keep"` → `"modify"`.
- Real auth via `lore auth login` (keychain) + returning-session check via `lore auth list --json`.
