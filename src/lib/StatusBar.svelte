<script lang="ts">
  import { session } from './session.svelte'
  import { repo, locks } from './repo.svelte'
  import Icon from './Icon.svelte'

  const mine = $derived(locks.list.filter((l) => l.holder === 'you').length)
  const others = $derived(locks.list.filter((l) => l.holder !== 'you').length)
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
</style>
