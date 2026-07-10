<script lang="ts">
  import { untrack } from 'svelte'
  import { api } from './api'
  import { session } from './session.svelte'
  import { repo, commit, setLock, discardFile } from './repo.svelte'
  import { composeCommitMessage } from './commitMessage'
  import { formatDelta } from './sizeFormat'
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
  const query = $derived(filter.trim().toLowerCase())
  const shown = $derived(query ? files.filter((f) => f.path.toLowerCase().includes(query)) : files)
  const branch = $derived(repo.status?.branch ?? 'main')
  const stagedCount = $derived(files.filter((f) => staged.has(f.path)).length)

  // Default: everything staged. Re-sync only when the SET of paths actually
  // changes — locks/sizes enrichment replaces `files` (new array, same paths)
  // every ~400ms, which must not re-check boxes the user just unchecked.
  const pathKey = $derived(files.map((f) => f.path).join('\n'))
  $effect(() => {
    pathKey
    staged = new Set(untrack(() => files).map((f) => f.path))
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
    const exclude = files.filter((f) => !staged.has(f.path)).map((f) => f.path)
    await commit(composeCommitMessage(message, description), exclude)
    message = ''
    description = ''
  }

  let ctxMenu = $state<{ x: number; y: number; path: string } | null>(null)

  function ctxItems(path: string) {
    const f = files.find((x) => x.path === path)
    const abs = `${session.config.currentRepo}/${path}`
    const wrap = (fn: () => void | Promise<void>) => async () => {
      try { await fn() } catch (e) { toastError('Action failed', e) }
    }
    const items: { label: string; icon?: string; danger?: boolean; run: () => void }[] = []
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
  <div class="colhead">Changes <span class="n">{query ? `${shown.length} of ${files.length} files` : `${files.length} ${files.length === 1 ? 'file' : 'files'}`}</span></div>

  <input class="filter" bind:value={filter} placeholder="Filter files" />

  <div class="filelist">
    {#if repo.busy === 'status' && !repo.status}
      <p class="muted pad">Scanning…</p>
    {:else if files.length === 0}
      <div class="empty muted"><p>No local changes.</p></div>
    {:else if shown.length === 0}
      <p class="muted pad">No files match.</p>
    {:else}
      <ul>
        {#each shown as f (f.path)}
          {@const d = formatDelta(f)}
          <li class="file" class:sel={f.path === selectedPath}
              oncontextmenu={(e) => { e.preventDefault(); ctxMenu = { x: e.clientX, y: e.clientY, path: f.path } }}>
            <input type="checkbox" checked={staged.has(f.path)} onchange={() => toggle(f.path)} title="Stage this file" aria-label="Stage {f.path}" />
            <div class="rowmain" role="button" tabindex="0"
                 onclick={() => onselect(f.path)}
                 onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect(f.path) } }}>
              <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>
              {#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}
              <span class="path"><span class="dir">{dir(f.path)}</span>{base(f.path)}</span>
              {#if d}<span class="delta">{d}</span>{/if}
              {#if f.lockedBy === 'you'}
                <span class="lock"><Icon name="lock" size={11} /> you</span>
              {:else if f.lockedBy}
                <span class="lock other"><Icon name="lock" size={11} /> {f.lockedBy}</span>
              {:else if f.isBinary}
                <span class="bin">bin</span>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="composer">
    <input bind:value={message} placeholder="Summary (required)" disabled={!!repo.busy} />
    <textarea rows="2" placeholder="Description" bind:value={description} disabled={!!repo.busy}></textarea>
    <button class="accent" onclick={doCommit} disabled={!!repo.busy || !message.trim() || stagedCount === 0}>
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
  .filter { display: block; margin: 8px 12px; width: calc(100% - 24px); padding: 6px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .pad { padding: 8px 12px; }
  .filelist { flex: 1; overflow: auto; }
  .filelist ul { list-style: none; margin: 0; padding: 4px 0; }
  .file { display: flex; align-items: center; gap: 8px; padding: 2px 12px; }
  .file:hover { background: var(--panel); }
  .file.sel { background: var(--accent-soft); }
  .file input { width: 14px; height: 14px; accent-color: var(--accent); flex-shrink: 0; margin: 0; }
  .rowmain { flex: 1; display: flex; align-items: center; gap: 8px; min-width: 0; cursor: pointer; padding: 5px 0; }
  .tag { width: 1.1em; text-align: center; font-weight: 500; flex-shrink: 0; }
  .rowthumb { width: 20px; height: 20px; border-radius: 4px; object-fit: cover; flex: none; }
  .tag.added { color: var(--added); } .tag.modified { color: var(--modified); } .tag.deleted { color: var(--deleted); }
  .path { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; font-size: 12.5px; }
  .dir { color: var(--text-muted); }
  .delta { flex-shrink: 0; font-size: 10.5px; font-family: var(--font-mono); color: var(--text-muted); }
  .lock { display: inline-flex; align-items: center; gap: 4px; flex-shrink: 0; font-size: 10.5px; background: var(--accent-soft); color: var(--accent-text); border-radius: 999px; padding: 1px 7px; }
  .lock.other { background: var(--panel); color: var(--text-muted); }
  .bin { flex-shrink: 0; font-size: 10px; padding: 1px 5px; border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted); }
  .empty { flex: 1; display: grid; place-items: center; }
  .composer { display: flex; flex-direction: column; gap: 8px; padding: 10px; border-top: 1px solid var(--border); background: var(--bg-elev); }
  .composer textarea { resize: none; }
  .composer button.accent { display: flex; align-items: center; justify-content: center; gap: 8px; }
  .cf { font-size: 11px; opacity: .8; }
</style>
