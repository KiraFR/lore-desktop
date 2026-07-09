<script lang="ts">
  import { session } from './session.svelte'
  import { repo, sync, push } from './repo.svelte'
  import { initialsFor } from './identity'
  import Icon from './Icon.svelte'
  import BranchMenu from './BranchMenu.svelte'
  import RepoSwitcher from './RepoSwitcher.svelte'
  import AvatarMenu from './AvatarMenu.svelte'

  const repoName = $derived(session.config.currentRepo?.split(/[\\/]/).pop() || 'Select a repository')
  const initials = $derived(initialsFor(session.config.displayName, session.identity?.email))
  let repoOpen = $state(false)
  let repoZoneEl = $state<HTMLDivElement>()
  let menuOpen = $state(false)
  let zoneEl = $state<HTMLDivElement>()
  let avatarOpen = $state(false)
  let avatarZoneEl = $state<HTMLDivElement>()

  // Close a menu when clicking anywhere outside its zone (button + popover).
  $effect(() => {
    if (!repoOpen) return
    function onDoc(e: PointerEvent) {
      if (repoZoneEl && !repoZoneEl.contains(e.target as Node)) repoOpen = false
    }
    document.addEventListener('pointerdown', onDoc)
    return () => document.removeEventListener('pointerdown', onDoc)
  })
  $effect(() => {
    if (!menuOpen) return
    function onDoc(e: PointerEvent) {
      if (zoneEl && !zoneEl.contains(e.target as Node)) menuOpen = false
    }
    document.addEventListener('pointerdown', onDoc)
    return () => document.removeEventListener('pointerdown', onDoc)
  })
  $effect(() => {
    if (!avatarOpen) return
    function onDoc(e: PointerEvent) {
      if (avatarZoneEl && !avatarZoneEl.contains(e.target as Node)) avatarOpen = false
    }
    document.addEventListener('pointerdown', onDoc)
    return () => document.removeEventListener('pointerdown', onDoc)
  })
</script>

<header class="titlebar">
  <div class="repozone" bind:this={repoZoneEl}>
    <button class="zone" class:open={repoOpen} onclick={() => (repoOpen = !repoOpen)} title="Switch repository">
      <Icon name="folder" size={16} />
      <div class="lbl"><span class="cap">Current repository</span><span class="val">{repoName}</span></div>
      <Icon name={repoOpen ? 'chevronUp' : 'chevronDown'} size={14} />
    </button>
    {#if repoOpen}<RepoSwitcher onclose={() => (repoOpen = false)} />{/if}
  </div>

  {#if session.config.currentRepo}
    <div class="branchzone" bind:this={zoneEl}>
      <button class="zone" class:open={menuOpen} onclick={() => (menuOpen = !menuOpen)} title="Current branch">
        <Icon name="branch" size={16} />
        <div class="lbl"><span class="cap">Current branch</span><span class="val">{repo.status?.branch ?? '…'}</span></div>
        <Icon name={menuOpen ? 'chevronUp' : 'chevronDown'} size={14} />
      </button>
      {#if menuOpen}<BranchMenu onclose={() => (menuOpen = false)} />{/if}
    </div>
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

  <div class="avatarzone" bind:this={avatarZoneEl}>
    <button class="avatar" class:open={avatarOpen} onclick={() => (avatarOpen = !avatarOpen)} title="Account">{initials}</button>
    {#if avatarOpen}<AvatarMenu onclose={() => (avatarOpen = false)} />{/if}
  </div>
</header>

<style>
  .titlebar { display: flex; align-items: center; gap: 8px; height: 48px; padding: 0 10px; background: var(--bg-elev); border-bottom: 1px solid var(--border); position: relative; z-index: 20; }
  .zone { display: flex; align-items: center; gap: 8px; height: 34px; max-width: 220px; }
  .zone.open { background: var(--accent-soft); border-color: var(--accent); }
  .repozone { position: relative; }
  .branchzone { position: relative; }
  .lbl { display: flex; flex-direction: column; line-height: 1.15; min-width: 0; text-align: left; }
  .cap { font-size: 10.5px; color: var(--text-muted); }
  .val { font-size: 13px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .action { display: flex; align-items: center; gap: 6px; height: 32px; }
  .action .count { font-size: 11px; color: var(--text-muted); }
  .action .count.on { color: var(--on-accent); opacity: .85; }
  .avatarzone { position: relative; }
  .avatar { width: 30px; height: 30px; border-radius: 50%; padding: 0; background: var(--accent-soft); color: var(--accent); border: none; font-size: 11px; font-weight: 500; }
  .avatar.open { outline: 2px solid var(--accent); }
</style>
