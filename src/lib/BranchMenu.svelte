<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { refreshStatus, refreshBranches, branches, repo } from './repo.svelte'
  import { setView } from './ui.svelte'
  import { confirmAction } from './confirm'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'
  import { groupBranches } from './branchGrouping'
  import { formatAheadBehind } from './branchInfoCache'

  let { onclose }: { onclose: () => void } = $props()

  let filter = $state('')
  let creating = $state(false)
  let newName = $state('')
  let busy = $state(false)

  const currentName = $derived(branches.list.find((b) => b.current)?.name ?? 'main')
  const rows = $derived(groupBranches(branches.list, filter))
  const branchCount = $derived(rows.reduce((n, r) => (r.kind === 'branch' ? n + 1 : n), 0))
  const curAB = $derived(formatAheadBehind({ ahead: repo.status?.localAhead ?? 0, behind: repo.status?.remoteAhead ?? 0 }))

  const LANE = ['#3067d4', '#3fb950', '#d29922', '#a371f7', '#ec6a5e']

  // Virtualize the list so a huge branch set stays smooth; compact when few.
  let listEl = $state<HTMLDivElement>()
  let listScroll = $state(0)
  const ROW_H = 34
  const listHeight = $derived(Math.min(rows.length * ROW_H, 238))
  const winFirst = $derived(Math.max(0, Math.floor(listScroll / ROW_H) - 4))
  const winLast = $derived(Math.min(rows.length, Math.ceil((listScroll + listHeight) / ROW_H) + 4))
  const windowRows = $derived(rows.slice(winFirst, winLast))
  function onListScroll() { if (listEl) listScroll = listEl.scrollTop }
  $effect(() => { filter; listScroll = 0; if (listEl) listEl.scrollTop = 0 })

  // Show the cached list immediately (no flash), then refresh on open so a menu
  // opened long after the last focus/sync is still current.
  $effect(() => { refreshBranches() })

  async function switchTo(name: string) {
    const p = session.config.currentRepo
    if (!p || busy) return
    busy = true
    try {
      await api.switchBranch(p, name)
      await refreshStatus()
      onclose()
    } catch (e) {
      toastError('Switch failed', e)
    } finally {
      busy = false
    }
  }

  async function create() {
    const p = session.config.currentRepo
    if (!p || !newName.trim() || busy) return
    busy = true
    try {
      await api.createBranch(p, newName.trim(), currentName)
      await refreshStatus()
      onclose()
    } catch (e) {
      toastError('Create failed', e)
    } finally {
      busy = false
    }
  }

  async function archive(name: string) {
    const p = session.config.currentRepo
    if (!p || busy) return
    const ok = await confirmAction(
      `Archive branch "${name}"? It disappears from branch lists; nothing is deleted.`,
      'Archive branch',
    )
    if (!ok) return
    busy = true
    try {
      await api.archiveBranch(p, name)
      await refreshBranches()
    } catch (e) {
      toastError('Archive failed', e)
    } finally {
      busy = false
    }
  }

  function mergeInto() { setView('merge'); onclose() }
</script>

<div class="menu">
  <input class="search" bind:value={filter} placeholder="Filter branches" />
  <div class="sec">Branches · {branchCount.toLocaleString()}{#if curAB} · {currentName} {curAB}{/if}</div>
  <div class="list" bind:this={listEl} onscroll={onListScroll} style="height:{listHeight}px">
    <div class="listvp" style="height:{rows.length * ROW_H}px">
      {#each windowRows as r, k (r.kind === 'header' ? '§' + r.label : r.branch.name)}
        <div class="rowwrap" style="top:{(winFirst + k) * ROW_H}px; height:{ROW_H}px">
          {#if r.kind === 'header'}
            <div class="subsec">{r.label}</div>
          {:else}
            {@const b = r.branch}
            <button class="item" class:cur={b.current} class:remote={b.location === 'remote'}
                    onclick={() => (b.current ? onclose() : switchTo(b.name))} disabled={busy}>
              <span class="dot" style="background:{LANE[(winFirst + k) % LANE.length]}"></span>
              <span class="bn">{b.name}</span>
              {#if b.current}<Icon name="check" size={14} />{/if}
            </button>
            {#if !b.current && b.location !== 'remote'}
              <!-- Archive reste locale : pas d'extension de la surface d'écriture aux
                   branches remote-only dans ce lot read-only. -->
              <button class="arch" title="Archive (hides from lists; nothing is deleted)"
                      onclick={() => archive(b.name)} disabled={busy}>Archive</button>
            {/if}
          {/if}
        </div>
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
  .subsec { display: flex; align-items: flex-end; height: 100%; padding: 0 12px 5px; font-size: 10px; text-transform: uppercase; letter-spacing: .04em; color: var(--text-dim); border-top: 1px solid var(--border); }
  .item.remote { opacity: .65; }
  .list { overflow-y: auto; overflow-x: hidden; }
  .listvp { position: relative; }
  .rowwrap { position: absolute; left: 0; right: 0; }
  .item { position: static; width: 100%; height: 100%; display: flex; align-items: center; gap: 9px; padding: 0 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .item:hover:not(:disabled) { background: var(--panel-hover); border: none; }
  .arch { position: absolute; top: 50%; right: 8px; transform: translateY(-50%); display: none; padding: 2px 6px; font-size: 10px; line-height: 1.3; color: var(--text-muted); background: var(--panel); border: 1px solid var(--border); border-radius: 5px; }
  .rowwrap:hover .arch { display: block; }
  .arch:hover:not(:disabled) { color: var(--text); background: var(--panel-hover); }
  .item.cur { color: var(--accent-text); }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .bn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item.cur :global(svg) { margin-left: auto; color: var(--accent-text); }
  .div { height: 1px; background: var(--border); margin: 6px 0; }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover { background: var(--panel-hover); border: none; }
  .action :global(svg) { color: var(--text-muted); }
  .createrow { display: flex; gap: 6px; padding: 4px 12px; }
  .createrow input { flex: 1; padding: 6px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .mk { padding: 6px 10px; font-size: 12px; background: var(--accent); color: var(--on-accent); border: none; border-radius: 6px; }
</style>
