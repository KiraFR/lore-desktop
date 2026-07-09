<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { repo, locks, refreshLocks, setLock } from './repo.svelte'
  import { listThumbs, requestThumb } from './thumbs.svelte'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'
  import ContextMenu from './ContextMenu.svelte'

  $effect(() => { session.config.currentRepo; refreshLocks() })

  // Queue row thumbnails for previewable locked images.
  $effect(() => {
    for (const l of locks.list) requestThumb(l.path)
  })

  const base = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
  const dir = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }
  const iconFor = (p: string) => (/\.(uasset|umap|png|fbx|wav)$/i.test(p) ? 'image' : 'file')

  let locking = $state(false)

  /** Absolute picked path → repo-relative ('/'-separated), or null if outside the repo. */
  function toRepoRelative(absPath: string, repoRoot: string): string | null {
    const norm = (p: string) => p.replaceAll('\\', '/').replace(/\/+$/, '')
    const abs = norm(absPath)
    const root = norm(repoRoot)
    if (!abs.toLowerCase().startsWith(root.toLowerCase() + '/')) return null
    return abs.slice(root.length + 1)
  }

  async function lockNewFile() {
    const root = session.config.currentRepo
    if (!root || locking) return
    const picked = await api.pickRepoFile(root)
    if (!picked) return // cancelled
    const rel = toRepoRelative(picked, root)
    if (!rel) {
      toastError('Not in this repository', new Error(picked))
      return
    }
    locking = true
    try {
      await setLock(rel, true) // setLock toasts its own failures + refreshes
    } finally {
      locking = false
    }
  }

  let ctxMenu = $state<{ x: number; y: number; path: string } | null>(null)

  function ctxItems(path: string) {
    const l = locks.list.find((x) => x.path === path)
    const abs = `${session.config.currentRepo}/${path}`
    const wrap = (fn: () => void | Promise<void>) => async () => {
      try { await fn() } catch (e) { toastError('Action failed', e) }
    }
    const items: { label: string; icon?: string; danger?: boolean; run: () => void }[] = [
      { label: 'Reveal in File Explorer', icon: 'folder', run: wrap(() => api.revealPath(abs)) },
      { label: 'Open file', icon: 'external', run: wrap(() => api.openPath(abs)) },
      { label: 'Copy path', icon: 'file', run: wrap(() => navigator.clipboard.writeText(path)) },
      { label: 'Copy full path', run: wrap(() => navigator.clipboard.writeText(abs)) },
    ]
    if (l?.holder === 'you') {
      items.push({ label: 'Unlock', icon: 'lock', run: wrap(() => setLock(path, false)) })
    }
    return items
  }
</script>

<div class="locks">
  <div class="lhead">
    <span class="title"><Icon name="lock" size={16} /> Locks <span class="count">{locks.list.length} held</span></span>
    <button class="ghost" onclick={lockNewFile} disabled={locking || !!repo.busy}>{locking ? 'Locking…' : '+ Lock a file…'}</button>
  </div>

  {#if locks.list.length === 0}
    <div class="empty muted">No files are locked.</div>
  {:else}
    <div class="list" role="list">
      {#each locks.list as l (l.path)}
        <div class="lrow" role="listitem"
             oncontextmenu={(e) => { e.preventDefault(); ctxMenu = { x: e.clientX, y: e.clientY, path: l.path } }}>
          {#if listThumbs.get(l.path)}
            <img class="rowthumb" src={listThumbs.get(l.path)} alt="" />
          {:else}
            <span class="fi"><Icon name={iconFor(l.path)} size={17} /></span>
          {/if}
          <span class="path"><span class="dir">{dir(l.path)}</span>{base(l.path)}</span>
          <span class="holder" class:you={l.holder === 'you'}>{l.holder}</span>
          <span class="when">{l.when}</span>
          {#if l.holder === 'you'}
            <button class="mini" onclick={() => setLock(l.path, false)} disabled={!!repo.busy}>Unlock</button>
          {:else}
            <span class="mini-space"></span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if ctxMenu}
    <ContextMenu x={ctxMenu.x} y={ctxMenu.y} items={ctxItems(ctxMenu.path)} onclose={() => (ctxMenu = null)} />
  {/if}
</div>

<style>
  .locks { flex: 1; overflow: auto; padding: 18px; max-width: 760px; }
  .lhead { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
  .title { display: inline-flex; align-items: center; gap: 8px; font-size: 14px; }
  .title :global(svg) { color: var(--text-muted); }
  .count { font-size: 11px; color: var(--text-dim); }
  .lhead .ghost { margin-left: auto; }
  .empty { padding: 24px; text-align: center; }
  .list { display: flex; flex-direction: column; gap: 8px; }
  .lrow { display: flex; align-items: center; gap: 11px; padding: 11px 12px; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); }
  .fi { color: var(--text-muted); display: inline-flex; flex-shrink: 0; }
  .rowthumb { width: 20px; height: 20px; border-radius: 4px; object-fit: cover; flex: none; }
  .path { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; font-size: 12.5px; }
  .dir { color: var(--text-muted); }
  .holder { flex-shrink: 0; font-size: 11px; border-radius: 999px; padding: 2px 9px; background: var(--panel-hover); color: var(--text-muted); }
  .holder.you { background: var(--accent-soft); color: var(--accent-text); }
  .when { flex-shrink: 0; font-size: 11px; color: var(--text-dim); width: 82px; text-align: right; }
  .mini { flex-shrink: 0; padding: 3px 10px; font-size: 11px; }
  .mini-space { width: 64px; flex-shrink: 0; }
</style>
