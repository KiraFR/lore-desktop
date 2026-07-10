<script lang="ts">
  import { session, signOut } from './session.svelte'
  import { repo, locks } from './repo.svelte'
  import { setView } from './ui.svelte'
  import { chipFor } from './statusChip'
  import Icon from './Icon.svelte'

  const mine = $derived(locks.list.filter((l) => l.holder === 'you').length)
  const others = $derived(locks.list.filter((l) => l.holder !== 'you').length)
  const offline = $derived(repo.status ? !repo.status.remoteAvailable : false)
  const expired = $derived(repo.status ? repo.status.remoteAvailable && !repo.status.remoteAuthorized : false)
  const chip = $derived(chipFor(repo.status))
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
  {#if chip?.kind === 'merge'}
    <button class="chip merge" onclick={() => setView('merge')} title="A merge is waiting for conflict resolution — click to resume it">
      <Icon name="branch" size={12} /> Merge in progress — resume
    </button>
  {:else if chip?.kind === 'staged'}
    <span class="chip" title="An interrupted commit or merge left a staged state; it will be picked up by the next commit or merge.">
      <Icon name="info" size={12} /> Staged state pending
    </span>
  {/if}
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

<style>
  .statusbar { display: flex; align-items: center; gap: 6px; height: 26px; padding: 0 12px; background: var(--bg-elev); border-top: 1px solid var(--border); font-size: 12px; color: var(--text-muted); }
  .item { display: inline-flex; align-items: center; gap: 5px; }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .dot.warn { background: var(--modified); }
  .dot.bad { background: var(--deleted); }
  .warn { color: var(--modified); }
  .bad { color: var(--deleted); }
  .mini { margin-left: 6px; padding: 1px 8px; font-size: 11px; }
  .chip { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; padding: 1px 8px; border-radius: 999px; background: var(--panel); color: var(--text-muted); border: 1px solid var(--border); }
  .chip.merge { background: var(--warn-bg); color: var(--warn-text); border-color: transparent; cursor: pointer; }
</style>
