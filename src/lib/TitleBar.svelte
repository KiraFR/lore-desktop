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
