<script lang="ts">
  import { api } from './api'
  import { session, setDisplayName, setTheme } from './session.svelte'
  import { updates, checkNow } from './updates.svelte'
  import { resolveTheme } from './theme'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'

  let { onclose }: { onclose: () => void } = $props()

  // --- Account: display name (relocated from AvatarMenu) ---
  let name = $state(session.config.displayName ?? '')
  const email = $derived(session.identity?.email ?? null)
  async function saveName() {
    if ((session.config.displayName ?? '') === name.trim()) return
    await setDisplayName(name)
  }

  // --- Appearance (relocated) ---
  const theme = $derived(resolveTheme(session.config.theme))

  // --- Clones: shared store (relocated) ---
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
    storeOn = target
    try {
      if (target) await api.sharedStoreEnable(serverUrl)
      else await api.sharedStoreDisable()
    } catch (e) {
      storeOn = prev
      toastError(target ? "Couldn't enable the shared store" : "Couldn't disable the shared store", e)
    }
  }

  // --- Support: updates ---
  const upd = $derived(updates.state)
  const updateHint = $derived(
    upd.kind === 'checking' ? 'Checking…'
      : upd.kind === 'upToDate' ? "You're up to date"
        : upd.kind === 'available' ? `v${upd.version} available — install from the status bar`
          : upd.kind === 'downloading' ? `Downloading… ${upd.pct}%`
            : upd.kind === 'ready' ? 'Restarting…'
              : upd.kind === 'error' ? `Check failed — ${upd.message}`
                : 'In-app updates')

  // --- Support: logs ---
  let logsPath = $state<string | null>(null)
  $effect(() => { api.logfileLocation().then((p) => { logsPath = p }).catch(() => { logsPath = null }) })
  async function openLogs() {
    try {
      const p = logsPath ?? (await api.logfileLocation())
      logsPath = p
      await api.openPath(p)
    } catch (e) {
      toastError("Couldn't open the logs folder", e)
    }
  }

  // Close on Escape / pointerdown outside (pattern AboutRepo).
  let panelEl = $state<HTMLDivElement>()
  $effect(() => {
    function onDoc(e: PointerEvent) { if (panelEl && !panelEl.contains(e.target as Node)) onclose() }
    function onKey(e: KeyboardEvent) { if (e.key === 'Escape') { e.stopPropagation(); onclose() } }
    document.addEventListener('pointerdown', onDoc)
    document.addEventListener('keydown', onKey)
    return () => { document.removeEventListener('pointerdown', onDoc); document.removeEventListener('keydown', onKey) }
  })
</script>

<div class="scrim">
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="Preferences">
    <div class="title"><Icon name="settings" size={16} /> Preferences</div>

    <div class="sec">Account</div>
    <label class="field">
      <span class="lbl">Display name</span>
      <input bind:value={name} placeholder="e.g. Jimmy D." onblur={saveName}
             onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); saveName() } }} />
    </label>
    {#if email}<div class="row"><span class="lbl">Email</span><span class="val">{email}</span></div>{/if}

    <div class="sec">Appearance</div>
    <div class="row">
      <span class="lbl">Theme</span>
      <div class="seg">
        <button class:active={theme === 'dark'} onclick={() => setTheme('dark')}>Dark</button>
        <button class:active={theme === 'light'} onclick={() => setTheme('light')}>Light</button>
      </div>
    </div>

    <div class="sec">Clones</div>
    <label class="row toggle" title="A shared object store lets every clone on this machine reuse the same on-disk objects instead of each keeping a full copy — saves disk space and speeds up new clones.">
      <span class="lbl">Use shared store<span class="hint">Clones reuse one local object store — saves disk</span></span>
      <input type="checkbox" checked={storeOn === true} disabled={storeOn === null || !session.config.serverUrl} onchange={toggleStore} />
    </label>

    <div class="sec">Support</div>
    <div class="row">
      <span class="lbl">Version {updates.appVersion ?? '…'}<span class="hint" class:err={upd.kind === 'error'}>{updateHint}</span></span>
      <button class="ghostbtn" onclick={checkNow} disabled={upd.kind === 'checking' || upd.kind === 'downloading' || upd.kind === 'ready'}>
        {upd.kind === 'checking' ? 'Checking…' : 'Check for updates'}
      </button>
    </div>
    <div class="row">
      <span class="lbl">Logs<span class="hint">{logsPath ?? 'CLI log directory'}</span></span>
      <button class="ghostbtn" onclick={openLogs}>Open logs</button>
    </div>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: rgba(0, 0, 0, .35); z-index: 90; display: grid; place-items: center; }
  .panel { width: 460px; max-width: calc(100vw - 40px); background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); padding: 14px 16px 16px; }
  .title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 500; margin-bottom: 8px; }
  .title :global(svg) { color: var(--text-muted); }
  .sec { font-size: 10px; text-transform: uppercase; letter-spacing: .04em; color: var(--text-dim); margin: 14px 0 6px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 5px 0; font-size: 12.5px; }
  .field { display: flex; flex-direction: column; gap: 4px; padding: 2px 0; }
  .field input { width: 100%; padding: 7px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .lbl { display: flex; flex-direction: column; min-width: 0; color: var(--text); }
  .hint { font-size: 10.5px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hint.err { color: var(--deleted); }
  .val { font-family: var(--font-mono); font-size: 12px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; }
  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 7px; overflow: hidden; flex: none; }
  .seg button { padding: 3px 12px; font-size: 11.5px; border: none; border-radius: 0; background: transparent; color: var(--text-muted); }
  .seg button.active { background: var(--accent); color: var(--on-accent); }
  .seg button:hover:not(.active) { background: var(--panel-hover); color: var(--text); }
  .toggle input { accent-color: var(--accent); flex: none; }
  .ghostbtn { flex: none; padding: 4px 10px; font-size: 11.5px; color: var(--text-muted); background: var(--bg); border: 1px solid var(--border); border-radius: 6px; }
  .ghostbtn:hover { color: var(--text); background: var(--panel-hover); }
</style>
