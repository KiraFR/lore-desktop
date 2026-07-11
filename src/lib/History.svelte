<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { repo, history, refreshHistory, loadMoreHistory, undoCommit } from './repo.svelte'
  import { listThumbs, requestThumb } from './thumbs.svelte'
  import { initialsFor } from './identity'
  import { confirmAction } from './confirm'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'
  import ContextMenu from './ContextMenu.svelte'
  import HistoryFilePreview from './HistoryFilePreview.svelte'
  import { toggleFilePath, selectionAfterCommitChange, isLocalTip } from './historySelection'
  import type { CommitFile } from './types'

  // Commits + selection live in the shared `history` store so leaving and
  // re-entering the History view keeps them (no remount reload / loading flash).
  const commits = $derived(history.commits)

  let glistEl = $state<HTMLDivElement>()
  let scrollTop = $state(0)
  let viewH = $state(560)

  const selected = $derived(commits.find((c) => c.id === history.selectedId) ?? null)
  const loading = $derived(!history.loaded && commits.length === 0)

  // A commit's files are fetched lazily on select (one diff vs its first parent),
  // never eagerly for every row. A same-commit refetch updates in place.
  let detailFiles = $state<CommitFile[]>([])
  let detailLoading = $state(false)
  let detailError = $state(false)
  let lastDetailId = ''

  // Selected commit-file path (opens the preview panel). Local state, NOT a
  // global store: it resets on commit change and evaporates on view leave.
  let previewPath = $state<string | null>(null)
  const previewFile = $derived(detailFiles.find((f) => f.path === previewPath) ?? null)

  $effect(() => {
    const c = selected
    const repoPath = session.config.currentRepo
    if (!c || !repoPath) { detailFiles = []; detailLoading = false; detailError = false; lastDetailId = ''; previewPath = null; return }
    const sameId = c.id === lastDetailId
    lastDetailId = c.id
    previewPath = selectionAfterCommitChange(sameId, previewPath)
    if (!sameId) { detailLoading = true; detailFiles = []; editing = false }
    detailError = false
    const parent = c.parents[0] ?? ''
    api
      .getCommitFiles(repoPath, c.id, parent)
      .then((files) => { if (selected?.id === c.id) detailFiles = files })
      .catch(() => { if (selected?.id === c.id) detailError = true })
      .finally(() => { if (selected?.id === c.id) detailLoading = false })
  })

  // Escape closes the preview panel — unless focus is in a text input (the
  // commit-message editor already binds Escape to cancel).
  $effect(() => {
    if (previewPath === null) return
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape' && !(e.target instanceof HTMLInputElement)) previewPath = null
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  // Queue row thumbnails for the selected commit's files (working-copy content).
  $effect(() => {
    for (const f of detailFiles) if (f.action !== 'delete') requestThumb(f.path)
  })

  const detailCounts = $derived({
    adds: detailFiles.filter((f) => f.action === 'add').length,
    mods: detailFiles.filter((f) => f.action === 'modify' || f.action === 'move' || f.action === 'copy').length,
    dels: detailFiles.filter((f) => f.action === 'delete').length,
  })

  // The last local (unpushed) commit can be undone — but only with a clean working
  // tree (no other pending changes), so the undo captures exactly the commit.
  const canUndo = $derived(
    !!selected && selected.id === history.commits[0]?.id &&
    (repo.status?.localAhead ?? 0) > 0 && selected.parents.length > 0 &&
    (repo.status?.files.length ?? 0) === 0,
  )

  async function doUndo() {
    if (!selected || !canUndo) return
    const ok = await confirmAction(
      `Undo the commit "${selected.message}"? Its changes go back to Changes (nothing is lost).`,
      'Undo commit',
    )
    if (ok) undoCommit(selected.parents[0])
  }

  // The last local (unpushed) commit's message can be amended. Unlike undo, this
  // doesn't need a clean working tree — only the message is rewritten.
  const canEdit = $derived(
    !!selected && selected.id === history.commits[0]?.id &&
    (repo.status?.localAhead ?? 0) > 0,
  )

  let editing = $state(false)
  let editMsg = $state('')

  function startEdit() {
    if (!selected || !canEdit) return
    editMsg = selected.message
    editing = true
  }

  async function saveEdit() {
    const path = session.config.currentRepo
    const msg = editMsg.trim()
    if (!path || !canEdit || !msg || repo.busy) return
    try {
      await api.amendCommit(path, msg)
      editing = false
      refreshHistory(true)
    } catch (e) {
      toastError('Amend failed', e)
    }
  }

  // On entering the view, refresh in the background: cached commits stay visible
  // (no blank), and the first-ever load shows the loading state via `loading`.
  $effect(() => { refreshHistory() })

  // Track the scroll viewport height so the window covers exactly what's visible.
  $effect(() => {
    if (!glistEl) return
    const measure = () => { viewH = glistEl!.clientHeight }
    measure()
    window.addEventListener('resize', measure)
    return () => window.removeEventListener('resize', measure)
  })

  const glyph: Record<string, { c: string; v: string }> = {
    add: { c: 'added', v: '+' }, modify: { c: 'modified', v: '~' }, delete: { c: 'deleted', v: '−' },
    move: { c: 'modified', v: 'R' }, copy: { c: 'modified', v: 'C' },
  }
  const dir = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }
  const base = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }

  const LANE_COLORS = ['#3067d4', '#3fb950', '#d29922', '#a371f7', '#ec6a5e']
  const laneColor = (l: number) => LANE_COLORS[l % LANE_COLORS.length]
  const ROW_H = 56, BASE_X = 18, LANE_GAP = 22, BUFFER = 6
  const laneX = (l: number) => BASE_X + l * LANE_GAP

  // History is newest-first, so the first `ahead` commits are the ones not yet
  // pushed to the server. Edges leaving those commits render dashed.
  const ahead = $derived(repo.status?.localAhead ?? 0)

  const idxMap = $derived(new Map(commits.map((c, i) => [c.id, i])))
  const maxLane = $derived(commits.reduce((m, c) => Math.max(m, c.lane), 0))
  const graphWidth = $derived(laneX(maxLane) + 16)
  const total = $derived(commits.length * ROW_H)
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - BUFFER))
  const last = $derived(Math.min(commits.length, Math.ceil((scrollTop + viewH) / ROW_H) + BUFFER))
  const windowCommits = $derived(commits.slice(first, last))

  const win = $derived.by(() => {
    const edges: { d: string; col: string; dashed: boolean }[] = []
    const dots: { x: number; y: number; color: string; merge: boolean }[] = []
    for (let i = first; i < last; i++) {
      const c = commits[i]
      if (!c) continue
      const y1 = (i - first) * ROW_H + ROW_H / 2
      dots.push({ x: laneX(c.lane), y: y1, color: laneColor(c.lane), merge: (c.parents?.length ?? 0) > 1 })
      for (const pid of c.parents ?? []) {
        const j = idxMap.get(pid)
        if (j === undefined) continue
        const y2 = (j - first) * ROW_H + ROW_H / 2
        const x1 = laneX(c.lane), x2 = laneX(commits[j].lane)
        const col = laneColor(Math.max(c.lane, commits[j].lane))
        const d = c.lane === commits[j].lane
          ? `M${x1} ${y1} L${x2} ${y2}`
          : `M${x1} ${y1} C${x1} ${(y1 + y2) / 2} ${x2} ${(y1 + y2) / 2} ${x2} ${y2}`
        edges.push({ d, col, dashed: i < ahead })
      }
    }
    return { edges, dots }
  })

  const PALETTE = [
    { bg: '#14304d', fg: '#7fb0ff' }, { bg: '#3a2b12', fg: '#e3b341' },
    { bg: '#132f22', fg: '#5fca9b' }, { bg: '#301a3d', fg: '#c79bff' },
  ]
  const meEmail = $derived(session.identity?.email ?? null)
  const isMe = (name: string) => name === 'you' || (meEmail !== null && name === meEmail)

  function avatar(name: string) {
    const initials = isMe(name)
      ? initialsFor(session.config.displayName, meEmail)
      : name.split(/[\s._@-]+/).filter(Boolean).map((w) => w[0]).join('').slice(0, 2).toUpperCase() || '?'
    let h = 0; for (let i = 0; i < name.length; i++) h += name.charCodeAt(i)
    return { initials, ...PALETTE[h % PALETTE.length] }
  }
  // Compact author label for inline text: 'you' for the signed-in user, else the
  // email local part (the full address shows in the avatar's hover tooltip).
  const shortName = (name: string) => (isMe(name) ? 'you' : name.includes('@') ? name.split('@')[0] : name)

  function onScroll() {
    if (!glistEl) return
    scrollTop = glistEl.scrollTop
    if (glistEl.scrollTop + glistEl.clientHeight > commits.length * ROW_H - viewH * 2) loadMoreHistory()
  }

  let ctxMenu = $state<{ x: number; y: number; path: string } | null>(null)

  function ctxItems(path: string) {
    const abs = `${session.config.currentRepo}/${path}`
    const wrap = (fn: () => void | Promise<void>) => async () => {
      try { await fn() } catch (e) { toastError('Action failed', e) }
    }
    return [
      { label: 'Reveal in File Explorer', icon: 'folder', run: wrap(() => api.revealPath(abs)) },
      { label: 'Open file', icon: 'external', run: wrap(() => api.openPath(abs)) },
      { label: 'Copy path', icon: 'file', run: wrap(() => navigator.clipboard.writeText(path)) },
      { label: 'Copy full path', run: wrap(() => navigator.clipboard.writeText(abs)) },
    ]
  }
</script>

<section class="history">
  <div class="leftcol">
    <div class="ghead">History <span class="cnt">{commits.length.toLocaleString()} commits</span></div>
    <div class="glist" bind:this={glistEl} onscroll={onScroll}>
      {#if loading && !commits.length}
        <p class="muted pad">Loading history…</p>
      {:else}
        <div class="viewport" style="height:{total}px">
          <svg class="graph" style="top:{first * ROW_H}px" width={graphWidth} height={(last - first) * ROW_H} fill="none">
            {#each win.edges as e}<path d={e.d} stroke={e.col} stroke-width="2" stroke-dasharray={e.dashed ? '4 3' : undefined} />{/each}
            {#each win.dots as dt}
              {#if dt.merge}
                <circle cx={dt.x} cy={dt.y} r="6" fill="var(--bg)" stroke={dt.color} stroke-width="2" />
              {:else}
                <circle cx={dt.x} cy={dt.y} r="4.5" fill={dt.color} />
              {/if}
            {/each}
          </svg>
          {#each windowCommits as c, k (c.id)}
            {@const i = first + k}
            {@const av = avatar(c.author)}
            <div class="grow" class:sel={c.id === history.selectedId} role="button" tabindex="0"
                 style="top:{i * ROW_H}px; height:{ROW_H}px; padding-left:{graphWidth + 10}px"
                 onclick={() => (history.selectedId = c.id)}
                 onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); history.selectedId = c.id } }}>
              {#if c.head}<span class="headpill" style="color:{laneColor(c.lane)};border-color:{laneColor(c.lane)}55;background:{laneColor(c.lane)}1f">{c.head}</span>{/if}
              <span class="ava" style="background:{av.bg};color:{av.fg}" title={c.author}>{av.initials}</span>
              <span class="cmid"><span class="cmsg">{c.message}</span><span class="csub" title={new Date(c.whenMs).toLocaleString()}>{shortName(c.author)} · {c.when}</span></span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <div class="detail">
    {#if selected}
      {@const av = avatar(selected.author)}
      <header class="dh">
        <span class="ava lg" style="background:{av.bg};color:{av.fg}" title={selected.author}>{av.initials}</span>
        <div><div class="dwho">{shortName(selected.author)}</div><div class="rev" title={new Date(selected.whenMs).toLocaleString()}>{selected.when} · #{selected.rev} · {selected.id}</div></div>
        {#if canEdit}
          <button class="undo" onclick={startEdit} disabled={!!repo.busy || editing} title="Rewrite this commit's message">
            <Icon name="edit" size={13} /> Edit message
          </button>
        {/if}
        {#if canUndo}
          <button class="undo" onclick={doUndo} disabled={!!repo.busy} title="Undo this commit — its changes go back to Changes">
            <Icon name="history" size={13} /> Undo commit
          </button>
        {/if}
      </header>
      {#if editing && canEdit}
        <div class="editrow">
          <input bind:value={editMsg} disabled={!!repo.busy} placeholder="Commit message"
                 onkeydown={(e) => { if (e.key === 'Enter') saveEdit(); else if (e.key === 'Escape') editing = false }} />
          <button class="save" onclick={saveEdit} disabled={!!repo.busy || !editMsg.trim()}>Save</button>
          <button class="cancel" onclick={() => (editing = false)}>Cancel</button>
        </div>
      {:else}
        <p class="dmsg">{selected.message}</p>
      {/if}
      <div class="fchg">
        Files changed · {detailFiles.length} {detailFiles.length === 1 ? 'file' : 'files'}
        <span class="counts">{#if detailCounts.adds}<span class="a">+{detailCounts.adds}</span>{/if}{#if detailCounts.mods}<span class="m">~{detailCounts.mods}</span>{/if}{#if detailCounts.dels}<span class="d">−{detailCounts.dels}</span>{/if}</span>
      </div>
      {#if detailLoading}
        <p class="floading muted">Loading files…</p>
      {:else if detailError}
        <p class="floading muted">Couldn't load files.</p>
      {:else if detailFiles.length === 0}
        <p class="floading muted">No file changes.</p>
      {:else}
        <ul class="fl">
          {#each detailFiles as f (f.path)}
            <li class:sel={f.path === previewPath} role="button" tabindex="0"
                onclick={() => (previewPath = toggleFilePath(previewPath, f.path))}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); previewPath = toggleFilePath(previewPath, f.path) } }}
                oncontextmenu={(e) => { e.preventDefault(); ctxMenu = { x: e.clientX, y: e.clientY, path: f.path } }}>
              <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>{#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}<span class="path"><span class="fdir">{dir(f.path)}</span>{base(f.path)}</span>
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <div class="empty muted"><p>Select a commit.</p></div>
    {/if}
  </div>

  {#if previewFile && selected}
    <HistoryFilePreview file={previewFile} isTip={isLocalTip(selected.id, commits)}
                        onclose={() => (previewPath = null)} />
  {/if}

  {#if ctxMenu}
    <ContextMenu x={ctxMenu.x} y={ctxMenu.y} items={ctxItems(ctxMenu.path)} onclose={() => (ctxMenu = null)} />
  {/if}
</section>

<style>
  .history { flex: 1; display: flex; overflow: hidden; min-width: 0; }
  .leftcol { width: 400px; flex-shrink: 0; display: flex; flex-direction: column; min-height: 0; border-right: 1px solid var(--border); }
  .ghead { flex: none; padding: 11px 14px; border-bottom: 1px solid var(--border); font-size: 12px; color: var(--text); display: flex; align-items: center; gap: 8px; }
  .ghead .cnt { color: var(--text-dim); font-size: 11px; }
  .glist { flex: 1; overflow-y: auto; overflow-x: hidden; }
  .pad { padding: 10px 14px; }
  .viewport { position: relative; }
  .graph { position: absolute; left: 8px; overflow: visible; z-index: 1; pointer-events: none; }
  .grow { position: absolute; left: 0; right: 0; display: flex; align-items: center; gap: 9px; padding-right: 14px; cursor: pointer; }
  .grow:hover { background: var(--panel); }
  .grow.sel { background: var(--accent-soft); }
  .headpill { font-size: 10.5px; padding: 1px 8px; border-radius: 20px; border: 1px solid; font-weight: 500; white-space: nowrap; flex: none; }
  .ava { width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; font-size: 10px; font-weight: 500; flex: none; }
  .ava.lg { width: 30px; height: 30px; font-size: 11px; }
  .cmid { min-width: 0; flex: 1; display: flex; flex-direction: column; gap: 1px; }
  .cmsg { font-size: 12.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .csub { font-size: 11px; color: var(--text-muted); }
  .counts { font-family: var(--font-mono); font-size: 11px; white-space: nowrap; flex: none; display: inline-flex; gap: 6px; }
  .counts .a { color: var(--added); } .counts .m { color: var(--modified); } .counts .d { color: var(--deleted); }
  .detail { flex: 1; overflow: auto; padding: 16px 18px; min-width: 0; }
  .dh { display: flex; align-items: center; gap: 11px; margin-bottom: 12px; }
  .dwho { font-size: 13px; font-weight: 500; }
  .rev { font-size: 11px; color: var(--text-muted); font-family: var(--font-mono); }
  .undo { display: inline-flex; align-items: center; gap: 5px; padding: 4px 10px; font-size: 11.5px; flex-shrink: 0; }
  .dh .undo:first-of-type { margin-left: auto; } /* whichever action shows first hugs the right edge */
  .dmsg { font-size: 13.5px; margin: 0 0 14px; }
  .editrow { display: flex; gap: 6px; margin: 0 0 14px; }
  .editrow input { flex: 1; min-width: 0; padding: 6px 8px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12.5px; }
  .editrow .save { padding: 6px 10px; font-size: 12px; background: var(--accent); color: var(--on-accent); border: none; border-radius: 6px; }
  .editrow .cancel { padding: 6px 10px; font-size: 12px; }
  .fchg { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: .04em; margin: 18px 0 8px; display: flex; align-items: center; gap: 10px; }
  .floading { font-size: 12.5px; padding: 6px 0; }
  .fl { list-style: none; margin: 0; padding: 0; }
  .fl li { display: flex; align-items: center; gap: 8px; padding: 5px 6px; margin: 0 -6px; border-radius: 6px; font-size: 12.5px; cursor: pointer; }
  .fl li:hover { background: var(--panel); }
  .fl li.sel { background: var(--accent-soft); }
  .rowthumb { width: 20px; height: 20px; border-radius: 4px; object-fit: cover; flex: none; }
  .tag { width: 1.1em; text-align: center; font-weight: 500; flex-shrink: 0; }
  .tag.added { color: var(--added); } .tag.modified { color: var(--modified); } .tag.deleted { color: var(--deleted); }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fdir { color: var(--text-muted); }
  .empty { height: 100%; display: grid; place-items: center; }
</style>
