<script lang="ts">
  import { untrack } from 'svelte'
  import { api } from './api'
  import { session } from './session.svelte'
  import { repo, commit, setLock, discardFile } from './repo.svelte'
  import { chipFor } from './statusChip'
  import { summaryParts } from './statusSummary'
  import { composeCommitMessage } from './commitMessage'
  import { formatDelta } from './sizeFormat'
  import { partitionByLock, filterByQuery } from './changesPartition'
  import { stepPath, rangePaths, stagePartition } from './changesKeyboard'
  import type { ChangedFile } from './types'
  import { listThumbs, requestThumb } from './thumbs.svelte'
  import { confirmAction } from './confirm'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'
  import ContextMenu from './ContextMenu.svelte'

  let { selectedPath, onselect }: { selectedPath: string | null; onselect: (p: string) => void } = $props()

  let message = $state('')
  let description = $state('')
  let staged = $state(new Set<string>())
  let filter = $state('')

  const glyph: Record<string, { c: string; v: string }> = {
    add: { c: 'added', v: '+' }, modify: { c: 'modified', v: '~' }, delete: { c: 'deleted', v: '−' },
    move: { c: 'modified', v: 'R' }, copy: { c: 'modified', v: 'C' },
  }
  const dir = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }
  const base = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }

  const files = $derived(repo.status?.files ?? [])
  const parts = $derived(partitionByLock(files))
  const shownCommittable = $derived(filterByQuery(parts.committable, filter))
  const shownLocked = $derived(filterByQuery(parts.lockedByOthers, filter))
  const shownCount = $derived(shownCommittable.length + shownLocked.length)
  const branch = $derived(repo.status?.branch ?? 'main')
  const summary = $derived(summaryParts(repo.status?.summary, repo.status?.ignoredCount ?? 0))
  const stagedCount = $derived(parts.committable.filter((f) => staged.has(f.path)).length)
  const behind = $derived(chipFor(repo.status)?.kind === 'behind')

  // Multi-selection (independent of the staging checkboxes). `multi` is the
  // set of highlighted rows; `anchorPath` remembers the last plain click so
  // shift+click can select a range in DISPLAYED order.
  let multi = $state(new Set<string>())
  let anchorPath: string | null = null
  const shown = $derived([...shownCommittable, ...shownLocked])

  function selectSingle(path: string) {
    multi = new Set([path])
    anchorPath = path
    onselect(path)
  }

  function rowClick(e: MouseEvent, path: string) {
    if (e.shiftKey && anchorPath !== null) {
      const range = rangePaths(shown.map((f) => f.path), anchorPath, path)
      if (range) {
        multi = new Set(range)
        onselect(path)
        return
      }
      // Anchor filtered out of view — fall through to a plain click.
    } else if (e.ctrlKey || e.metaKey) {
      const next = new Set(multi)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
        onselect(path)
      }
      multi = next
      return
    }
    selectSingle(path)
  }

  // Escape collapses a multi-selection back to the single current selection.
  function collapseMulti() {
    if (multi.size <= 1) return
    const sp = selectedPath !== null && multi.has(selectedPath) ? selectedPath : [...multi][0]
    multi = new Set([sp])
    anchorPath = sp
  }

  let listEl = $state<HTMLDivElement>()

  function scrollRowIntoView(path: string) {
    listEl?.querySelector(`[data-path="${path.replace(/"/g, '\\"')}"]`)?.scrollIntoView({ block: 'nearest' })
  }

  // Keyboard selection over the DISPLAYED order. The handler sits on the list
  // container: the filter input and the commit composer live outside it, so
  // their keystrokes never reach here; the staging checkboxes DO live inside,
  // hence the target guard.
  function listKeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement) return
    if (e.key === 'Escape') { collapseMulti(); return }
    const order = shown.map((f) => f.path)
    if (order.length === 0) return
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
      e.preventDefault()
      multi = new Set(order)
      return
    }
    let target: string | null
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      target = stepPath(order, selectedPath, e.key === 'ArrowDown' ? 1 : -1)
    } else if (e.key === 'Home' || e.key === 'End') {
      target = e.key === 'Home' ? order[0] : order[order.length - 1]
    } else {
      return
    }
    e.preventDefault()
    if (target === null) return
    if (e.shiftKey && anchorPath !== null) {
      // Same range semantics as shift+click: the anchor stays, the active end
      // moves with the arrows.
      const range = rangePaths(order, anchorPath, target)
      if (range) {
        multi = new Set(range)
        onselect(target)
        scrollRowIntoView(target)
        return
      }
    }
    selectSingle(target)
    scrollRowIntoView(target)
  }

  const committablePathKey = $derived(parts.committable.map((f) => f.path).join('\n'))
  // Default: every committable file staged. Teammate-locked files are NEVER
  // staged — exclusion by construction (doCommit excludes everything unstaged),
  // so there is no way to commit them from the app. Rebuilt only when the
  // committable path set actually changes (enrichment merges keep selection,
  // and so does a teammate locking/unlocking a file elsewhere in the list).
  //
  // Invariant preserved across rebuilds: a brand-new path arrives staged;
  // a path we already knew about keeps whatever staged/unstaged state the
  // user left it in; a path that disappeared is simply dropped.
  let prevCommittable = new Set<string>()
  $effect(() => {
    committablePathKey
    const committable = untrack(() => parts.committable).map((f) => f.path)
    const prevStaged = untrack(() => staged)
    staged = new Set(committable.filter((p) => !prevCommittable.has(p) || prevStaged.has(p)))
    prevCommittable = new Set(committable)
  })

  // Keep the multi-selection in sync with the file set: purge paths that
  // disappeared on a status refresh, and if the selection ends up empty while
  // files remain, fall back to the single current selection. Keyed on the path
  // set so enrichment merges and user clicks don't churn it.
  const allPathKey = $derived(files.map((f) => f.path).join('\n'))
  $effect(() => {
    allPathKey
    const valid = new Set(untrack(() => files).map((f) => f.path))
    const cur = untrack(() => multi)
    const kept = [...cur].filter((p) => valid.has(p))
    let next = kept.length !== cur.size ? new Set(kept) : cur
    if (next.size === 0 && valid.size > 0) {
      const sp = untrack(() => selectedPath)
      if (sp !== null && valid.has(sp)) next = new Set([sp])
    }
    if (next !== cur) multi = next
    if (anchorPath !== null && !valid.has(anchorPath)) anchorPath = null
  })

  // Queue row thumbnails for previewable images (deleted files have no working copy).
  $effect(() => {
    for (const f of files) if (f.action !== 'delete') requestThumb(f.path)
  })

  function toggle(path: string) {
    const next = new Set(staged)
    next.has(path) ? next.delete(path) : next.add(path)
    staged = next
  }

  async function doCommit() {
    // Everything not staged is excluded — that covers unchecked committables
    // AND every teammate-locked file (never in `staged`, by construction).
    const exclude = files.filter((f) => !staged.has(f.path)).map((f) => f.path)
    await commit(composeCommitMessage(message, description), exclude)
    message = ''
    description = ''
  }

  let ctxMenu = $state<{ x: number; y: number; path: string } | null>(null)

  function openCtxMenu(e: MouseEvent, path: string) {
    e.preventDefault()
    // Right-click outside the multi-selection selects that row alone first.
    if (!(multi.size > 1 && multi.has(path))) {
      multi = new Set([path])
      anchorPath = path
    }
    ctxMenu = { x: e.clientX, y: e.clientY, path }
  }

  type CtxItem = { label: string; icon?: string; danger?: boolean; run: () => void }

  // Bulk actions over the multi-selection. Each loop awaits sequentially (each
  // action refreshes status on its own — acceptable) and keeps going past
  // per-item failures. No Reveal/Open in grouped mode.
  function ctxItemsMulti(): CtxItem[] {
    const sel = files.filter((f) => multi.has(f.path))
    const paths = sel.map((f) => f.path)
    const lockable = sel.filter((f) => !f.lockedBy).map((f) => f.path)
    const unlockable = sel.filter((f) => f.lockedBy === 'you').map((f) => f.path)
    const forEachPath = (ps: string[], fn: (p: string) => void | Promise<void>) => async () => {
      for (const p of ps) {
        try { await fn(p) } catch (e) { toastError('Action failed', e) }
      }
    }
    const items: CtxItem[] = []
    // Staging is purely front-side (the local `staged` set) — no API call.
    // stagePartition walks the committable paths only, so teammate-locked
    // rows in the selection can never end up staged.
    const { toStage, toUnstage } = stagePartition(multi, parts.committable.map((f) => f.path), staged)
    if (toStage.length > 0) {
      items.push({ label: `Stage ${toStage.length} file${toStage.length === 1 ? '' : 's'}`, icon: 'check', run: () => { staged = new Set([...staged, ...toStage]) } })
    }
    if (toUnstage.length > 0) {
      items.push({ label: `Unstage ${toUnstage.length} file${toUnstage.length === 1 ? '' : 's'}`, run: () => { const next = new Set(staged); for (const p of toUnstage) next.delete(p); staged = next } })
    }
    if (lockable.length > 0) {
      items.push({ label: `Lock ${lockable.length} file${lockable.length === 1 ? '' : 's'}`, icon: 'lock', run: forEachPath(lockable, (p) => setLock(p, true)) })
    }
    if (unlockable.length > 0) {
      items.push({ label: `Unlock ${unlockable.length} file${unlockable.length === 1 ? '' : 's'}`, icon: 'lock', run: forEachPath(unlockable, (p) => setLock(p, false)) })
    }
    items.push({
      label: `Copy ${paths.length} paths`, icon: 'file',
      run: async () => {
        try { await navigator.clipboard.writeText(paths.join('\n')) } catch (e) { toastError('Action failed', e) }
      },
    })
    items.push({
      label: `Discard ${paths.length} files…`, icon: 'history', danger: true,
      run: async () => {
        const ok = await confirmAction(`Discard changes to ${paths.length} files? This can't be undone.`, 'Discard changes')
        if (ok) await forEachPath(paths, (p) => discardFile(p))()
      },
    })
    return items
  }

  function ctxItems(path: string): CtxItem[] {
    if (multi.size > 1 && multi.has(path)) return ctxItemsMulti()
    const f = files.find((x) => x.path === path)
    const abs = `${session.config.currentRepo}/${path}`
    const wrap = (fn: () => void | Promise<void>) => async () => {
      try { await fn() } catch (e) { toastError('Action failed', e) }
    }
    const items: CtxItem[] = []
    if (f && !(f.lockedBy && f.lockedBy !== 'you')) {
      // Committable rows only — a teammate-locked file is never stageable.
      if (staged.has(path)) items.push({ label: 'Unstage', run: () => toggle(path) })
      else items.push({ label: 'Stage', icon: 'check', run: () => toggle(path) })
    }
    if (f?.action !== 'delete') {
      // A deleted file has no working copy to reveal or open.
      items.push({ label: 'Reveal in File Explorer', icon: 'folder', run: wrap(() => api.revealPath(abs)) })
      items.push({ label: 'Open file', icon: 'external', run: wrap(() => api.openPath(abs)) })
    }
    items.push({ label: 'Copy path', icon: 'file', run: wrap(() => navigator.clipboard.writeText(path)) })
    items.push({ label: 'Copy full path', run: wrap(() => navigator.clipboard.writeText(abs)) })
    if (f?.action !== 'delete') {
      // No working copy to hold a lock on once the file is deleted.
      if (f?.lockedBy === 'you') {
        items.push({ label: 'Unlock', icon: 'lock', run: wrap(() => setLock(path, false)) })
      } else if (!f?.lockedBy) {
        items.push({ label: 'Lock', icon: 'lock', run: wrap(() => setLock(path, true)) })
      }
    }
    items.push({
      label: 'Discard changes…', icon: 'history', danger: true,
      run: wrap(async () => {
        const ok = await confirmAction(`Discard changes to ${path}? This can't be undone.`, 'Discard changes')
        if (ok) discardFile(path)
      }),
    })
    return items
  }
</script>

<section class="changes">
  <div class="colhead">Changes
    <span class="n">{filter.trim() ? `${shownCount} of ${files.length} files` : `${files.length} ${files.length === 1 ? 'file' : 'files'}`}{multi.size > 1 ? ` · ${multi.size} selected` : ''}</span>
    {#if summary.length > 0}
      <span class="sum" aria-label="Change counters">
        {#each summary as p (p.cls)}<span class="p {p.cls}">{p.text}</span>{/each}
      </span>
    {/if}
  </div>

  <input class="filter" bind:value={filter} placeholder="Filter files" />

  <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
  <div class="filelist" role="listbox" aria-label="Changed files" aria-multiselectable="true"
       tabindex="0" bind:this={listEl} onkeydown={listKeydown}>
    {#if repo.busy === 'status' && !repo.status}
      <p class="muted pad">Scanning…</p>
    {:else if files.length === 0}
      <div class="empty muted"><p>No local changes.</p></div>
    {:else if shownCount === 0}
      <p class="muted pad">No files match.</p>
    {:else}
      {#if parts.committable.length === 0 && files.length > 0}
        <p class="muted pad">All changed files are locked by teammates.</p>
      {/if}
      {#if shownCommittable.length > 0}
        <ul>
          {#each shownCommittable as f (f.path)}
            <li class="file" role="option" class:sel={multi.has(f.path)} data-path={f.path} aria-selected={multi.has(f.path)}
                oncontextmenu={(e) => openCtxMenu(e, f.path)}>
              <input type="checkbox" checked={staged.has(f.path)} onchange={() => toggle(f.path)} title="Stage this file" aria-label="Stage {f.path}" />
              {@render fileRow(f, false)}
            </li>
          {/each}
        </ul>
      {/if}
      {#if shownLocked.length > 0}
        <div class="lockedhead" role="heading" aria-level="3" id="locked-head">
          <Icon name="lock" size={12} />
          <span>Locked by teammates ({shownLocked.length}) — excluded from commit</span>
        </div>
        <ul aria-labelledby="locked-head">
          {#each shownLocked as f (f.path)}
            <li class="file locked" role="option" class:sel={multi.has(f.path)} data-path={f.path} aria-selected={multi.has(f.path)}
                oncontextmenu={(e) => openCtxMenu(e, f.path)}>
              {@render fileRow(f, true)}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </div>

  {#snippet fileRow(f: ChangedFile, locked: boolean)}
    {@const d = formatDelta(f)}
    <div class="rowmain" role="button" tabindex="0"
         onclick={(e) => rowClick(e, f.path)}
         onkeydown={(e) => {
           if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectSingle(f.path) }
           else if (e.key === 'Escape') collapseMulti()
         }}>
      <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>
      {#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}
      <span class="path"><span class="dir">{dir(f.path)}</span>{base(f.path)}</span>
      {#if d}<span class="delta">{d}</span>{/if}
      {#if locked}
        <span class="lock other" aria-label="Locked by {f.lockedBy}"><Icon name="lock" size={11} /> {f.lockedBy}</span>
      {:else if f.lockedBy === 'you'}
        <span class="lock" aria-label="Locked by you"><Icon name="lock" size={11} /> you</span>
      {:else if f.isBinary}
        <span class="bin">bin</span>
      {/if}
    </div>
  {/snippet}

  <div class="composer">
    <input bind:value={message} placeholder="Summary (required)" disabled={!!repo.busy} />
    <textarea rows="2" placeholder="Description" bind:value={description} disabled={!!repo.busy}></textarea>
    <button class="accent" onclick={doCommit} disabled={!!repo.busy || !message.trim() || stagedCount === 0 || behind}
            title={behind ? 'Commit is disabled while behind the latest — sync back first' : undefined}>
      {repo.busy === 'commit' ? 'Committing…' : `Commit to ${branch}`}
      {#if stagedCount > 0}<span class="cf">{stagedCount} {stagedCount === 1 ? 'file' : 'files'}</span>{/if}
    </button>
  </div>

  {#if ctxMenu}
    <ContextMenu x={ctxMenu.x} y={ctxMenu.y} items={ctxItems(ctxMenu.path)} onclose={() => (ctxMenu = null)} />
  {/if}
</section>

<style>
  .changes { display: flex; flex-direction: column; width: 320px; flex-shrink: 0; overflow: hidden; border-right: 1px solid var(--border); }
  .colhead { padding: 11px 14px; border-bottom: 1px solid var(--border); font-size: 13px; color: var(--text); }
  .colhead .n { color: var(--text-dim); font-size: 12px; margin-left: 4px; }
  .sum { margin-left: 6px; font-size: 11px; font-family: var(--font-mono); }
  .sum .p { margin-right: 5px; }
  .sum .added { color: var(--added); }
  .sum .modified { color: var(--modified); }
  .sum .deleted { color: var(--deleted); }
  .sum .ignored { color: var(--text-muted); }
  .filter { display: block; margin: 8px 12px; width: calc(100% - 24px); padding: 6px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .pad { padding: 8px 12px; }
  .filelist { flex: 1; overflow: auto; }
  .filelist:focus-visible { outline: 1px solid var(--accent); outline-offset: -1px; }
  .filelist ul { list-style: none; margin: 0; padding: 4px 0; }
  .file { display: flex; align-items: center; gap: 8px; padding: 2px 12px; }
  .file:hover { background: var(--panel); }
  .file.sel { background: var(--accent-soft); }
  .file input { width: 14px; height: 14px; accent-color: var(--accent); flex-shrink: 0; margin: 0; }
  .rowmain { flex: 1; display: flex; align-items: center; gap: 8px; min-width: 0; cursor: pointer; padding: 5px 0; user-select: none; }
  .tag { width: 1.1em; text-align: center; font-weight: 500; flex-shrink: 0; }
  .rowthumb { width: 20px; height: 20px; border-radius: 4px; object-fit: cover; flex: none; }
  .tag.added { color: var(--added); } .tag.modified { color: var(--modified); } .tag.deleted { color: var(--deleted); }
  .path { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; font-size: 12.5px; }
  .dir { color: var(--text-muted); }
  .delta { flex-shrink: 0; font-size: 10.5px; font-family: var(--font-mono); color: var(--text-muted); }
  .lock { display: inline-flex; align-items: center; gap: 4px; flex-shrink: 0; font-size: 10.5px; background: var(--accent-soft); color: var(--accent-text); border-radius: 999px; padding: 1px 7px; }
  .lock.other { background: var(--panel); color: var(--text-muted); }
  .bin { flex-shrink: 0; font-size: 10px; padding: 1px 5px; border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted); }
  .lockedhead { display: flex; align-items: center; gap: 6px; padding: 10px 12px 4px; font-size: 11px; color: var(--warn-text); border-top: 1px solid var(--border); margin-top: 4px; }
  .file.locked .rowmain { opacity: .75; padding-left: 22px; }
  .empty { flex: 1; display: grid; place-items: center; }
  .composer { display: flex; flex-direction: column; gap: 8px; padding: 10px; border-top: 1px solid var(--border); background: var(--bg-elev); }
  .composer textarea { resize: none; }
  .composer button.accent { display: flex; align-items: center; justify-content: center; gap: 8px; }
  .cf { font-size: 11px; opacity: .8; }
</style>
