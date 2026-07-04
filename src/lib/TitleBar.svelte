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
