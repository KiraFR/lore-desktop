<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { addExistingRepo, cloneServerRepo } from './repoActions'
  import { toastError } from './toast'
  import { opProgress } from './opProgress.svelte'
  import { pct, cloneLabel } from './progress'
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
    if (busy) return
    busy = `clone:${entry.id}`
    try {
      await cloneServerRepo(entry)
    } finally {
      busy = ''
    }
  }

  $effect(() => { loadRepos() })
</script>

<div class="picker">
 <div class="inner">
  <h2>Open a repository</h2>

  <div class="card">
    <div><strong>Local working copy</strong><p class="muted small">Choose a folder you've already cloned.</p></div>
    <span class="spacer"></span>
    <button class="accent" onclick={openFolder} disabled={busy === 'open'}>
      {busy === 'open' ? 'Opening…' : 'Open folder…'}
    </button>
  </div>

  <h3>On {session.config.serverUrl}</h3>
  {#if loading}<p class="muted">Loading repositories…</p>{/if}
  <ul class="repos">
    {#each repos as r (r.id)}
      <li>
        <span class="ico"><Icon name="folder" size={16} /></span>
        <div class="meta"><strong>{r.name}</strong><p class="muted small mono">{r.id.slice(0, 12)}…</p></div>
        <span class="spacer"></span>
        <button onclick={() => cloneRepo(r)} disabled={!!busy}>
          {busy === `clone:${r.id}` ? cloneLabel(pct(opProgress.clone)) : 'Clone…'}
        </button>
        {#if busy === `clone:${r.id}`}
          {@const p = pct(opProgress.clone)}
          <span class="clonebar" class:indet={p === null} style="width: {p ?? 40}%"
                role={p === null ? undefined : 'progressbar'} aria-valuemin={p === null ? undefined : 0}
                aria-valuemax={p === null ? undefined : 100} aria-valuenow={p === null ? undefined : p}
                aria-hidden={p === null ? 'true' : undefined}></span>
        {/if}
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
  .repos li { display: flex; align-items: center; gap: 10px; padding: 10px 4px; border-bottom: 1px solid var(--border); position: relative; overflow: hidden; }
  .ico { color: var(--accent); }
  .meta { min-width: 0; }
  .clonebar { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--accent); transition: width .25s ease; }
  .clonebar.indet { animation: pickerslide 1.1s linear infinite; }
  @keyframes pickerslide { from { transform: translateX(-100%); } to { transform: translateX(350%); } }
</style>
