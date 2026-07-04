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
