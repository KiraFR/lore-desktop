<script lang="ts">
  import type { ChangedFile, DiffLine, PreviewData } from './types'
  import { repo, setLock, discardFile } from './repo.svelte'
  import { session } from './session.svelte'
  import { api } from './api'
  import { confirmAction } from './confirm'
  import Icon from './Icon.svelte'
  import MediaPreview from './MediaPreview.svelte'
  import FileHistorySection from './FileHistorySection.svelte'
  import { fmtSize } from './sizeFormat'
  import { typeName } from './fileTypes'

  let { file }: { file: ChangedFile | null } = $props()

  async function doDiscard(f: ChangedFile) {
    const ok = await confirmAction(`Discard changes to ${f.path}? This can't be undone.`, 'Discard changes')
    if (ok) discardFile(f.path)
  }

  let diff = $state<DiffLine[]>([])
  let diffLoading = $state(false)
  let diffError = $state(false)
  let lastDiffPath = ''

  // Fetch the unified diff whenever a text file is selected. A same-file refetch
  // (e.g. the window-focus refresh) updates the diff in place — no loading flicker.
  $effect(() => {
    const f = file
    if (!f || f.isBinary) { diff = []; diffError = false; diffLoading = false; lastDiffPath = ''; return }
    const repoPath = session.config.currentRepo
    if (!repoPath) { diff = []; diffLoading = false; return }
    const samePath = f.path === lastDiffPath
    lastDiffPath = f.path
    if (!samePath) { diffLoading = true; diff = [] }
    diffError = false
    api
      .getDiff(repoPath, f.path)
      .then((d) => { if (file?.path === f.path) diff = d })
      .catch(() => { if (file?.path === f.path) diffError = true })
      .finally(() => { if (file?.path === f.path) diffLoading = false })
  })

  // Working-copy preview (image thumbnail / audio / 3D) for binary files,
  // populated by MediaPreview; the Dimensions row below reads it.
  let preview = $state<PreviewData | null>(null)

  const baseName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
  const dirName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }

  const badge = $derived(
    file?.action === 'add' ? { t: 'Added', c: 'added' } :
    file?.action === 'delete' ? { t: 'Deleted', c: 'deleted' } :
    { t: 'Modified', c: 'modified' },
  )

  const sizeText = $derived(
    !file ? ''
    : file.action === 'delete' && file.oldSize != null ? fmtSize(file.oldSize)
    : file.action === 'modify' && file.oldSize != null ? `${fmtSize(file.oldSize)} → ${fmtSize(file.size)}`
    : fmtSize(file.size),
  )
</script>

<div class="preview">
  {#if !file}
    <div class="empty ph muted">
      <p>Select a file to preview.</p>
      <p class="dim small">Pick a file on the left to see its changes.</p>
    </div>
  {:else}
    <div class="body">
      <header class="head">
        <div class="ic"><Icon name={file.isBinary ? 'image' : 'file'} size={20} /></div>
        <div class="ttl">
          <div class="fn">{baseName(file.path)}</div>
          <div class="fp muted">{dirName(file.path)}</div>
        </div>
        <span class="badge {badge.c}">{badge.t}</span>
        <button class="discard" onclick={() => doDiscard(file)} disabled={!!repo.busy} title="Discard changes to this file">
          <Icon name="history" size={13} /> Discard
        </button>
      </header>

      {#if file.lockedBy && file.lockedBy !== 'you'}
        <div class="lockwarn" role="status">
          <Icon name="lock" size={14} />
          <span>Locked by {file.lockedBy} — excluded from commit while locked</span>
        </div>
      {/if}

      {#if file.isBinary}
        <MediaPreview path={file.path} action={file.action} bind:preview />
      {:else if diffLoading}
        <div class="textnote muted"><Icon name="file" size={22} /><p>Loading diff…</p></div>
      {:else if diffError}
        <div class="textnote muted"><Icon name="file" size={22} /><p>Couldn't load diff.</p></div>
      {:else if diff.length === 0}
        <div class="textnote muted"><Icon name="file" size={22} /><p>No text changes to show.</p></div>
      {:else}
        <div class="diff">
          {#each diff as line, i (i)}
            <div class="dl {line.kind}">
              <span class="ln">{line.oldLine ?? ''}</span>
              <span class="ln">{line.newLine ?? ''}</span>
              <span class="mk">{line.kind === 'add' ? '+' : line.kind === 'del' ? '-' : ''}</span>
              <span class="tx">{line.text}</span>
            </div>
          {/each}
        </div>
      {/if}

      <dl class="meta">
        <div><dt>Type</dt><dd>{typeName(file.path)}</dd></div>
        <div><dt>Size</dt><dd>{sizeText}</dd></div>
        <!-- Gated on isBinary: `preview` is only reset by MediaPreview while it
             is mounted, so a stale binary's dimensions would linger on a text file. -->
        {#if file.isBinary && preview?.width && preview?.height}
          <div><dt>Dimensions</dt><dd>{preview.width} × {preview.height}</dd></div>
        {/if}
        <div>
          <dt>Lock</dt>
          <dd>
            {#if file.lockedBy === 'you'}
              <span class="lockrow"><Icon name="lock" size={13} /> Locked by you</span>
              <button class="mini" onclick={() => setLock(file.path, false)} disabled={!!repo.busy}>Unlock</button>
            {:else if file.lockedBy}
              <span class="lockrow other"><Icon name="lock" size={13} /> Locked by {file.lockedBy}</span>
            {:else}
              <span class="muted">Not locked</span>
              <button class="mini" onclick={() => setLock(file.path, true)} disabled={!!repo.busy}>Lock</button>
            {/if}
          </dd>
        </div>
      </dl>

      <FileHistorySection path={file.path} />
    </div>
  {/if}
</div>

<style>
  .preview { flex: 1; overflow: auto; min-width: 0; }
  .empty { height: 100%; display: grid; place-items: center; text-align: center; padding: 20px; }
  .empty .small { font-size: 12px; margin-top: 4px; }
  .body { padding: 16px 18px; max-width: 720px; }
  .head { display: flex; align-items: center; gap: 11px; margin-bottom: 16px; }
  .lockwarn { display: flex; align-items: center; gap: 8px; background: var(--warn-bg); color: var(--warn-text); border-radius: 8px; padding: 9px 12px; font-size: 12px; margin: 0 0 14px; }
  .ic { width: 34px; height: 34px; border-radius: 8px; background: var(--panel); display: grid; place-items: center; color: var(--text-muted); flex-shrink: 0; }
  .ttl { min-width: 0; }
  .fn { font-size: 14px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fp { font-size: 11px; }
  .badge { margin-left: auto; border-radius: var(--radius); padding: 3px 9px; font-size: 11px; flex-shrink: 0; }
  .badge.modified { background: var(--warn-bg); color: var(--warn-text); }
  .badge.added { background: rgba(63, 185, 80, .15); color: var(--added); }
  .badge.deleted { background: rgba(248, 81, 73, .15); color: var(--deleted); }
  .discard { display: inline-flex; align-items: center; gap: 5px; padding: 3px 9px; font-size: 11px; color: var(--text-muted); flex-shrink: 0; }
  .discard:hover:not(:disabled) { color: var(--deleted); border-color: var(--deleted); }
  .textnote { display: flex; align-items: center; gap: 12px; padding: 22px; border: 1px dashed var(--border); border-radius: 8px; font-size: 12.5px; }
  .textnote p { margin: 0; }
  .diff { font-family: var(--font-mono); font-size: 12px; line-height: 1.55; border: 1px solid var(--border); border-radius: 8px; overflow-x: auto; margin: 4px 0; }
  .dl { display: flex; }
  .ln { flex: 0 0 44px; text-align: right; padding: 0 8px; color: var(--text-dim); user-select: none; }
  .mk { flex: 0 0 16px; text-align: center; color: var(--text-dim); user-select: none; }
  .tx { flex: 1; white-space: pre; padding-right: 12px; }
  .dl.add { background: rgba(63, 185, 80, .12); }
  .dl.add .mk, .dl.add .tx { color: var(--added); }
  .dl.del { background: rgba(248, 81, 73, .12); }
  .dl.del .mk, .dl.del .tx { color: var(--deleted); }
  .dl.context .tx { color: var(--text-muted); }
  .dl.hunk { background: var(--panel); }
  .dl.hunk .tx { color: var(--accent-text); }
  .meta { margin: 18px 0 0; }
  .meta > div { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 9px 0; border-top: 1px solid var(--border); font-size: 12.5px; }
  dt { color: var(--text-muted); }
  dd { margin: 0; display: inline-flex; align-items: center; gap: 10px; }
  .lockrow { display: inline-flex; align-items: center; gap: 6px; color: var(--accent-text); }
  .lockrow.other { color: var(--text-muted); }
  .mini { padding: 3px 10px; font-size: 11px; }
</style>
