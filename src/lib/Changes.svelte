<script lang="ts">
  import { repo, commit } from './repo.svelte'
  import { composeCommitMessage } from './commitMessage'
  import { listThumbs, requestThumb } from './thumbs.svelte'
  import Icon from './Icon.svelte'

  let { selectedPath, onselect }: { selectedPath: string | null; onselect: (p: string) => void } = $props()

  let message = $state('')
  let description = $state('')
  let staged = $state(new Set<string>())

  const glyph: Record<string, { c: string; v: string }> = {
    add: { c: 'added', v: '+' }, modify: { c: 'modified', v: '~' }, delete: { c: 'deleted', v: '−' },
    move: { c: 'modified', v: 'R' }, copy: { c: 'modified', v: 'C' },
  }
  const dir = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }
  const base = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }

  const files = $derived(repo.status?.files ?? [])
  const branch = $derived(repo.status?.branch ?? 'main')
  const stagedCount = $derived(files.filter((f) => staged.has(f.path)).length)

  // Default: everything staged. Re-sync whenever the file set changes.
  $effect(() => {
    staged = new Set(files.map((f) => f.path))
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
</script>

<section class="changes">
  <div class="colhead">Changes <span class="n">{files.length} {files.length === 1 ? 'file' : 'files'}</span></div>

  <div class="filelist">
    {#if repo.busy === 'status' && !repo.status}
      <p class="muted pad">Scanning…</p>
    {:else if files.length === 0}
      <div class="empty muted"><p>No local changes.</p></div>
    {:else}
      <ul>
        {#each files as f (f.path)}
          <li class="file" class:sel={f.path === selectedPath}>
            <input type="checkbox" checked={staged.has(f.path)} onchange={() => toggle(f.path)} title="Stage this file" aria-label="Stage {f.path}" />
            <div class="rowmain" role="button" tabindex="0"
                 onclick={() => onselect(f.path)}
                 onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect(f.path) } }}>
              <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>
              {#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}
              <span class="path"><span class="dir">{dir(f.path)}</span>{base(f.path)}</span>
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
</section>

<style>
  .changes { display: flex; flex-direction: column; width: 320px; flex-shrink: 0; overflow: hidden; border-right: 1px solid var(--border); }
  .colhead { padding: 11px 14px; border-bottom: 1px solid var(--border); font-size: 13px; color: var(--text); }
  .colhead .n { color: var(--text-dim); font-size: 12px; margin-left: 4px; }
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
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; font-size: 12.5px; }
  .dir { color: var(--text-muted); }
  .lock { margin-left: auto; display: inline-flex; align-items: center; gap: 4px; flex-shrink: 0; font-size: 10.5px; background: var(--accent-soft); color: var(--accent-text); border-radius: 999px; padding: 1px 7px; }
  .lock.other { background: var(--panel); color: var(--text-muted); }
  .bin { margin-left: auto; flex-shrink: 0; font-size: 10px; padding: 1px 5px; border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted); }
  .empty { flex: 1; display: grid; place-items: center; }
  .composer { display: flex; flex-direction: column; gap: 8px; padding: 10px; border-top: 1px solid var(--border); background: var(--bg-elev); }
  .composer textarea { resize: none; }
  .composer button.accent { display: flex; align-items: center; justify-content: center; gap: 8px; }
  .cf { font-size: 11px; opacity: .8; }
</style>
