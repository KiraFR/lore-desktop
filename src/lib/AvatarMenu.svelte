<script lang="ts">
  import { session, signOut } from './session.svelte'
  import { repo } from './repo.svelte'
  import { initialsFor, displayNameFor } from './identity'
  import Icon from './Icon.svelte'

  let { onclose, onpreferences }: { onclose: () => void; onpreferences: () => void } = $props()

  const email = $derived(session.identity?.email ?? null)
  const initials = $derived(initialsFor(session.config.displayName, email))
  const label = $derived(displayNameFor(session.config.displayName, email))

  async function doSignOut() {
    await signOut()
    onclose()
  }
</script>

<div class="menu">
  <div class="who">
    <span class="ava">{initials}</span>
    <div class="ids">
      <span class="nm">{label}</span>
      <span class="em">{email ?? 'Open a repository to load your identity'}</span>
    </div>
  </div>
  <div class="div"></div>
  <button class="action" onclick={() => { onclose(); onpreferences() }}>
    <Icon name="settings" size={15} /> Preferences…
  </button>
  <button class="action out" onclick={doSignOut} disabled={!!repo.busy}>
    <Icon name="external" size={15} /> Sign out
  </button>
</div>

<style>
  .menu { position: absolute; top: calc(100% + 6px); right: 0; width: 260px; background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); z-index: 50; overflow: hidden; padding: 8px 0; }
  .who { display: flex; align-items: center; gap: 10px; padding: 8px 14px 10px; }
  .ava { width: 34px; height: 34px; border-radius: 50%; background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; font-size: 12px; font-weight: 500; flex: none; }
  .ids { min-width: 0; display: flex; flex-direction: column; }
  .nm { font-size: 13px; font-weight: 500; }
  .em { font-size: 11.5px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .div { height: 1px; background: var(--border); margin: 2px 0 6px; }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 14px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover:not(:disabled) { background: var(--panel-hover); border: none; }
  .action.out { color: var(--deleted); }
  .action :global(svg) { color: currentColor; }
</style>
