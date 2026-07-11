<script lang="ts">
  import type { ChangedFile, DiffLine, FileRevision, PreviewData } from './types'
  import { repo, setLock, discardFile } from './repo.svelte'
  import { session } from './session.svelte'
  import { api } from './api'
  import { confirmAction } from './confirm'
  import Icon from './Icon.svelte'
  import AudioPlayer from './AudioPlayer.svelte'
  import ModelViewer from './ModelViewer.svelte'
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

  // Working-copy preview (image thumbnail / audio) for binary files. Same
  // anti-race pattern as the diff: check the selection still matches on arrival.
  let preview = $state<PreviewData | null>(null)
  let lastPreviewPath = ''

  $effect(() => {
    const f = file
    const repoPath = session.config.currentRepo
    if (!f || !f.isBinary || f.action === 'delete' || !repoPath) { preview = null; lastPreviewPath = ''; return }
    const same = f.path === lastPreviewPath
    lastPreviewPath = f.path
    if (!same) preview = null
    api.getPreview(repoPath, f.path)
      .then((p) => { if (file?.path === f.path) preview = p })
      .catch(() => { if (file?.path === f.path) preview = null })
  })

  // Per-asset revision timeline, fetched lazily on selection (anti-race).
  let fileHistory = $state<FileRevision[]>([])
  let fhLoading = $state(false)
  let fhError = $state(false)
  let lastFhPath = ''

  $effect(() => {
    const f = file
    const repoPath = session.config.currentRepo
    if (!f || !repoPath) { fileHistory = []; fhLoading = false; fhError = false; lastFhPath = ''; return }
    const same = f.path === lastFhPath
    lastFhPath = f.path
    if (!same) { fileHistory = []; fhLoading = true }
    fhError = false
    api.getFileHistory(repoPath, f.path)
      .then((revs) => { if (file?.path === f.path) fileHistory = revs })
      .catch(() => { if (file?.path === f.path) fhError = true })
      .finally(() => { if (file?.path === f.path) fhLoading = false })
  })

  const glyph: Record<string, { c: string; v: string }> = {
    add: { c: 'added', v: '+' }, modify: { c: 'modified', v: '~' }, delete: { c: 'deleted', v: '−' },
    move: { c: 'modified', v: 'R' }, copy: { c: 'modified', v: 'C' },
  }
  const authorLabel = (a: string) =>
    a === session.identity?.email ? 'you' : a.includes('@') ? a.split('@')[0] : a.slice(0, 8)

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
        {#if preview?.kind === 'audio' && preview.url}
          <AudioPlayer src={preview.url} name={baseName(file.path)} />
          <p class="note muted"><Icon name="info" size={14} /> Audio asset — plays the working copy.</p>
        {:else if preview?.kind === 'model' && preview.url}
          <ModelViewer url={preview.url} name={baseName(file.path)} />
          <p class="note muted"><Icon name="info" size={14} /> 3D preview of the working copy — drag to orbit, scroll to zoom.</p>
        {:else}
          <div class="cmp">
            {#if file.action !== 'add'}
              <figure class="cbox">
                <div class="thumb before"><Icon name="image" size={26} /></div>
                <figcaption>Before · previous revision</figcaption>
              </figure>
            {/if}
            {#if file.action !== 'delete'}
              <figure class="cbox">
                {#if preview?.kind === 'image' && preview.url}
                  <div class="thumb after img"><img src={preview.url} alt={baseName(file.path)} /></div>
                {:else}
                  <div class="thumb after"><Icon name="image" size={26} /></div>
                {/if}
                <figcaption class="aft">{file.action === 'add' ? 'New file' : 'After · working copy'}</figcaption>
              </figure>
            {/if}
          </div>
          {#if preview?.kind === 'image'}
            <p class="note muted"><Icon name="info" size={14} /> Previous-revision preview needs server support — working copy only.</p>
          {:else}
            <p class="note muted"><Icon name="info" size={14} /> Binary asset — visual compare, no text diff.</p>
          {/if}
        {/if}
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
        {#if preview?.width && preview?.height}
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

      <div class="fhhead">History{#if fileHistory.length} · {fileHistory.length} {fileHistory.length === 1 ? 'revision' : 'revisions'}{/if}</div>
      {#if fhLoading}
        <p class="fhnote muted">Loading history…</p>
      {:else if fhError}
        <p class="fhnote muted">Couldn't load file history.</p>
      {:else if fileHistory.length === 0}
        <p class="fhnote muted">No committed revisions yet.</p>
      {:else}
        <ul class="fhl">
          {#each fileHistory.slice(0, 30) as r (r.revision)}
            <li>
              <span class="tag {glyph[r.action]?.c}">{glyph[r.action]?.v ?? '?'}</span>
              <span class="frev">#{r.revisionNumber}</span>
              <span class="fmsg" title={r.message}>{r.message}</span>
              <span class="fwho">{authorLabel(r.author)}</span>
              <span class="fwhen" title={new Date(r.whenMs).toLocaleString()}>{r.when}</span>
              <span class="fsize">{fmtSize(r.size)}</span>
            </li>
          {/each}
        </ul>
        {#if fileHistory.length > 30}<p class="fhnote muted">…and {fileHistory.length - 30} more revisions</p>{/if}
      {/if}
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
  .cmp { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; }
  figure { margin: 0; }
  .thumb { height: 150px; border-radius: 8px; display: grid; place-items: center; color: var(--text-dim); border: 1px solid var(--border); }
  .thumb.before { background: #2b2f35; }
  .thumb.after { background: #33475f; }
  /* The box is a grid with auto rows, where a percentage height on the img
     resolves as auto (335×335 in a 335×149 box, then clipped). Absolute
     positioning sizes the img against the box itself instead. */
  .thumb.img { padding: 0; overflow: hidden; position: relative; background: repeating-conic-gradient(#2b2f35 0% 25%, #333a44 0% 50%) 50% / 24px 24px; }
  .thumb.img img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; }
  figcaption { font-size: 11px; color: var(--text-muted); margin-top: 7px; text-align: center; }
  figcaption.aft { color: var(--accent-text); }
  .note { display: flex; align-items: center; gap: 7px; font-size: 11px; margin: 12px 0 4px; }
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
  .tag { width: 1.1em; text-align: center; font-weight: 500; flex-shrink: 0; }
  .tag.added { color: var(--added); } .tag.modified { color: var(--modified); } .tag.deleted { color: var(--deleted); }
  .fhhead { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: .04em; margin: 20px 0 6px; }
  .fhnote { font-size: 12px; margin: 4px 0; }
  .fhl { list-style: none; margin: 0; padding: 0; }
  .fhl li { display: flex; align-items: center; gap: 9px; padding: 6px 0; border-top: 1px solid var(--border); font-size: 12.5px; }
  .frev { font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); flex: none; min-width: 28px; }
  .fmsg { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fwho { flex: none; font-size: 11px; color: var(--accent-text); }
  .fwhen { flex: none; font-size: 11px; color: var(--text-dim); }
  .fsize { flex: none; font-size: 11px; color: var(--text-muted); font-family: var(--font-mono); min-width: 58px; text-align: right; }
</style>
