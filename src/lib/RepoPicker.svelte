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
