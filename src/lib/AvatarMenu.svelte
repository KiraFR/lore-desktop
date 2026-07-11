<script lang="ts">
  import { session, signOut, setDisplayName, setTheme } from './session.svelte'
  import { repo } from './repo.svelte'
  import { initialsFor, displayNameFor } from './identity'
  import { resolveTheme } from './theme'
  import Icon from './Icon.svelte'
  import { api } from './api'
  import { toastError } from './toast'

  let { onclose }: { onclose: () => void } = $props()

  let name = $state(session.config.displayName ?? '')
  const theme = $derived(resolveTheme(session.config.theme))

  const email = $derived(session.identity?.email ?? null)
  const initials = $derived(initialsFor(session.config.displayName, email))
  const label = $derived(displayNameFor(session.config.displayName, email))

  async function saveName() {
    if ((session.config.displayName ?? '') === name.trim()) return
    await setDisplayName(name)
  }

  async function doSignOut() {
    await signOut()
    onclose()
  }

  // null = status not loaded yet (toggle disabled until then).
  let storeOn = $state<boolean | null>(null)

  $effect(() => {
    api.sharedStoreStatus()
      .then((s) => { storeOn = s.autoUse ?? s.exists })
      .catch(() => { storeOn = null })
  })

  async function toggleStore() {
    if (storeOn === null) return
    const serverUrl = session.config.serverUrl
    if (!serverUrl) return
    const target = !storeOn
    const prev = storeOn
    storeOn = target // optimistic — revert on failure
    try {
      if (target) await api.sharedStoreEnable(serverUrl)
      else await api.sharedStoreDisable()
    } catch (e) {
      storeOn = prev
      toastError(target ? "Couldn't enable the shared store" : "Couldn't disable the shared store", e)
    }
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
  <div class="field">
    <label for="dn">Display name</label>
    <input id="dn" bind:value={name} placeholder="e.g. Jimmy D." onblur={saveName}
           onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); saveName() } }} />
  </div>
  <label class="storetoggle"
         title="A shared object store lets every clone on this machine reuse the same on-disk objects instead of each keeping a full copy — saves disk space and speeds up new clones.">
    <span class="stlabel">
      Shared store
      <span class="sthint">Clones reuse one local object store — saves disk</span>
    </span>
    <input type="checkbox" checked={storeOn === true} disabled={storeOn === null || !session.config.serverUrl} onchange={toggleStore} />
  </label>
  <div class="appearance">
    <span class="aplabel">Appearance</span>
    <div class="seg">
      <button class:active={theme === 'dark'} onclick={() => setTheme('dark')}>Dark</button>
      <button class:active={theme === 'light'} onclick={() => setTheme('light')}>Light</button>
    </div>
  </div>
  <div class="div"></div>
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
  .field { padding: 0 14px 10px; display: flex; flex-direction: column; gap: 4px; }
  .field label { font-size: 10.5px; color: var(--text-dim); }
  .field input { width: 100%; padding: 6px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .div { height: 1px; background: var(--border); margin: 2px 0 6px; }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 14px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover:not(:disabled) { background: var(--panel-hover); border: none; }
  .action.out { color: var(--deleted); }
  .action :global(svg) { color: currentColor; }
  .storetoggle { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 0 14px 10px; font-size: 12px; color: var(--text); }
  .stlabel { display: flex; flex-direction: column; min-width: 0; }
  .sthint { font-size: 10.5px; color: var(--text-dim); }
  /* Override the global `input { width: 100% }` — an unconstrained checkbox
     stretches to the full row and squashes the label to nothing. */
  .storetoggle input { flex: none; width: auto; accent-color: var(--accent); }
  .appearance { display: flex; align-items: center; justify-content: space-between; padding: 0 14px 10px; font-size: 12px; color: var(--text); }
  .aplabel { color: var(--text); }
  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 7px; overflow: hidden; }
  .seg button { padding: 3px 12px; font-size: 11.5px; border: none; border-radius: 0; background: transparent; color: var(--text-muted); }
  .seg button.active { background: var(--accent); color: var(--on-accent); }
  .seg button:hover:not(.active) { background: var(--panel-hover); color: var(--text); }
</style>
