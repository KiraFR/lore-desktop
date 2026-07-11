<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { repo } from './repo.svelte'
  import { aboutRows } from './aboutFields'
  import type { RepositoryInfo } from './types'
  import Icon from './Icon.svelte'

  let { onclose }: { onclose: () => void } = $props()

  let info = $state<RepositoryInfo | null>(null)
  let copied = $state('')

  // Best-effort : les lignes locales (chemin, branche, révision) rendent tout de
  // suite ; les champs serveur se remplissent quand l'appel aboutit. Un échec
  // (hors-ligne) laisse simplement le panneau local-only — pas de toast.
  $effect(() => {
    const p = session.config.currentRepo
    if (!p) return
    api.getRepositoryInfo(p).then((r) => { info = r }).catch(() => { /* rows stay local-only */ })
  })

  const rows = $derived(aboutRows(info, {
    repoPath: session.config.currentRepo,
    serverUrl: session.config.serverUrl,
    branch: repo.status?.branch ?? null,
    revisionNumber: repo.status?.revisionNumber ?? null,
  }))

  // Fermeture Escape / pointerdown hors du panneau (pattern ContextMenu.svelte).
  let panelEl = $state<HTMLDivElement>()
  $effect(() => {
    function onDoc(e: PointerEvent) {
      if (panelEl && !panelEl.contains(e.target as Node)) onclose()
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') { e.stopPropagation(); onclose() }
    }
    document.addEventListener('pointerdown', onDoc)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('pointerdown', onDoc)
      document.removeEventListener('keydown', onKey)
    }
  })

  async function copy(value: string, label: string) {
    try {
      await navigator.clipboard.writeText(value)
      copied = label
      setTimeout(() => (copied = ''), 1500)
    } catch { /* clipboard denied — silent */ }
  }
</script>

<div class="scrim">
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="About repository">
    <div class="title"><Icon name="info" size={16} /> About repository</div>
    {#each rows as r (r.label)}
      <div class="row">
        <span class="lbl">{r.label}</span>
        <span class="val" title={r.value}>{r.value}</span>
        {#if r.copyable}
          <button class="mini" onclick={() => copy(r.value, r.label)}>{copied === r.label ? 'Copied' : 'Copy'}</button>
        {/if}
        {#if r.revealPath}
          <button class="mini" onclick={() => api.revealPath(r.revealPath!)}>Reveal</button>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: rgba(0, 0, 0, .35); z-index: 90; display: grid; place-items: center; }
  .panel { width: 420px; max-width: calc(100vw - 40px); background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); padding: 14px 16px 12px; }
  .title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 500; margin-bottom: 10px; }
  .title :global(svg) { color: var(--text-muted); }
  .row { display: flex; align-items: center; gap: 8px; padding: 5px 0; font-size: 12.5px; }
  .lbl { width: 110px; flex: none; color: var(--text-muted); font-size: 11.5px; }
  .val { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: var(--font-mono); font-size: 12px; }
  .mini { flex: none; padding: 2px 8px; font-size: 10.5px; color: var(--text-muted); background: var(--bg); border: 1px solid var(--border); border-radius: 5px; }
  .mini:hover { color: var(--text); background: var(--panel-hover); }
</style>
