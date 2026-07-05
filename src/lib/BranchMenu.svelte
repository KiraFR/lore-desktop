<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { refreshStatus } from './repo.svelte'
  import { setView } from './ui.svelte'
  import type { Branch } from './types'
  import Icon from './Icon.svelte'

  let { onclose }: { onclose: () => void } = $props()

  let branches = $state<Branch[]>([])
  let filter = $state('')
  let creating = $state(false)
  let newName = $state('')
  let busy = $state(false)

  const currentName = $derived(branches.find((b) => b.current)?.name ?? 'main')
  const shown = $derived(branches.filter((b) => b.name.toLowerCase().includes(filter.trim().toLowerCase())))

  const LANE = ['#3067d4', '#3fb950', '#d29922', '#a371f7', '#ec6a5e']

  // Virtualize the list so a huge branch set stays smooth; compact when few.
  let listEl = $state<HTMLDivElement>()
  let listScroll = $state(0)
  const ROW_H = 34
  const listHeight = $derived(Math.min(shown.length * ROW_H, 238))
  const winFirst = $derived(Math.max(0, Math.floor(listScroll / ROW_H) - 4))
  const winLast = $derived(Math.min(shown.length, Math.ceil((listScroll + listHeight) / ROW_H) + 4))
  const windowBranches = $derived(shown.slice(winFirst, winLast))
  function onListScroll() { if (listEl) listScroll = listEl.scrollTop }
  $effect(() => { filter; listScroll = 0; if (listEl) listEl.scrollTop = 0 })

  async function load() {
    const p = session.config.currentRepo
    if (!p) return
    branches = await api.getBranches(p)
  }
  $effect(() => { load() })

  async function switchTo(name: string) {
    const p = session.config.currentRepo
    if (!p || busy) return
    busy = true
    await api.switchBranch(p, name)
    await refreshStatus()
    busy = false
    onclose()
  }

  async function create() {
    const p = session.config.currentRepo
    if (!p || !newName.trim() || busy) return
    busy = true
    await api.createBranch(p, newName.trim(), currentName)
    await refreshStatus()
    busy = false
    onclose()
  }

  function mergeInto() { setView('merge'); onclose() }
</script>

<div class="menu">
  <input class="search" bind:value={filter} placeholder="Filter branches" />
  <div class="sec">Branches · {shown.length.toLocaleString()}</div>
  <div class="list" bind:this={listEl} onscroll={onListScroll} style="height:{listHeight}px">
    <div class="listvp" style="height:{shown.length * ROW_H}px">
      {#each windowBranches as b, k (b.name)}
        <button class="item" class:cur={b.current} style="top:{(winFirst + k) * ROW_H}px; height:{ROW_H}px"
                onclick={() => (b.current ? onclose() : switchTo(b.name))} disabled={busy}>
          <span class="dot" style="background:{LANE[(winFirst + k) % LANE.length]}"></span>
          <span class="bn">{b.name}</span>
          {#if b.current}<Icon name="check" size={14} />{:else}<span class="rev">#{b.rev}</span>{/if}
        </button>
      {/each}
    </div>
  </div>
  <div class="div"></div>
  {#if creating}
    <div class="createrow">
      <input bind:value={newName} placeholder="feature/my-change" disabled={busy} />
      <button class="mk" onclick={create} disabled={busy || !newName.trim()}>Create</button>
    </div>
  {:else}
    <button class="action" onclick={() => (creating = true)}><Icon name="plus" size={15} /> New branch…</button>
  {/if}
  <button class="action" onclick={mergeInto}><Icon name="merge" size={15} /> Merge a branch into {currentName}…</button>
</div>

<style>
  .menu { position: absolute; top: calc(100% + 6px); left: 0; width: 280px; background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); z-index: 50; overflow: hidden; padding: 8px 0; }
  .search { display: block; margin: 4px 10px 8px; width: calc(100% - 20px); padding: 7px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .sec { font-size: 10px; text-transform: uppercase; letter-spacing: .04em; color: var(--text-dim); padding: 2px 12px 5px; }
  .list { overflow-y: auto; overflow-x: hidden; }
  .listvp { position: relative; }
  .item { position: absolute; left: 0; right: 0; display: flex; align-items: center; gap: 9px; padding: 0 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .item:hover:not(:disabled) { background: var(--panel-hover); border: none; }
  .item.cur { color: var(--accent-text); }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .bn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item .rev { margin-left: auto; font-family: var(--font-mono); font-size: 11px; color: var(--text-dim); }
  .item.cur :global(svg) { margin-left: auto; color: var(--accent-text); }
  .div { height: 1px; background: var(--border); margin: 6px 0; }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover { background: var(--panel-hover); border: none; }
  .action :global(svg) { color: var(--text-muted); }
  .createrow { display: flex; gap: 6px; padding: 4px 12px; }
  .createrow input { flex: 1; padding: 6px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .mk { padding: 6px 10px; font-size: 12px; background: var(--accent); color: var(--on-accent); border: none; border-radius: 6px; }
</style>
