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
