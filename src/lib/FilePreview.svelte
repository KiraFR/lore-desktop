<script lang="ts">
  import type { ChangedFile, DiffLine, PreviewData } from './types'
  import { repo, setLock, discardFile } from './repo.svelte'
  import { session } from './session.svelte'
  import { api } from './api'
  import { confirmAction } from './confirm'
  import Icon from './Icon.svelte'
  import AudioPlayer from './AudioPlayer.svelte'

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

  const KB = 1024, MB = 1024 * 1024
  function fmtSize(n: number): string {
    if (n >= MB) return (n / MB).toFixed(1) + ' MB'
    if (n >= KB) return (n / KB).toFixed(1) + ' KB'
    return n + ' B'
  }
  const ext = (p: string) => { const i = p.lastIndexOf('.'); return i < 0 ? '' : p.slice(i + 1).toLowerCase() }
  const TYPES: Record<string, string> = {
    uasset: 'Unreal asset', umap: 'Level (map)', pak: 'Unreal package',
    cpp: 'C++ source', h: 'C++ header', cs: 'C# source', ini: 'Config', md: 'Markdown', json: 'JSON',
    png: 'Texture', tga: 'Texture', dds: 'Texture', tif: 'Texture', tiff: 'Texture', jpg: 'Texture', jpeg: 'Texture', webp: 'Texture',
    exr: 'HDR texture', hdr: 'HDR texture', psd: 'Photoshop document',
    fbx: 'Mesh', obj: 'Mesh', abc: 'Alembic cache', gltf: 'Mesh', glb: 'Mesh',
    blend: 'Blender scene', ma: 'Maya scene', mb: 'Maya scene', max: '3ds Max scene', ztl: 'ZBrush tool',
    sbs: 'Substance graph', sbsar: 'Substance archive', spp: 'Substance Painter project',
    wav: 'Audio', ogg: 'Audio', mp3: 'Audio', flac: 'Audio', bank: 'Audio bank',
    anim: 'Animation',
  }
  const typeName = (p: string) => TYPES[ext(p)] ?? (ext(p) ? ext(p).toUpperCase() + ' file' : 'File')
  const baseName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
  const dirName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }

  const badge = $derived(
    file?.action === 'add' ? { t: 'Added', c: 'added' } :
    file?.action === 'delete' ? { t: 'Deleted', c: 'deleted' } :
    { t: 'Modified', c: 'modified' },
  )

  const sizeText = $derived(
    file
      ? (file.oldSize != null && file.action === 'modify' ? `${fmtSize(file.oldSize)} → ${fmtSize(file.size)}` : fmtSize(file.size))
      : '',
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

      {#if file.isBinary}
        {#if preview?.kind === 'audio' && preview.url}
          <AudioPlayer src={preview.url} name={baseName(file.path)} />
          <p class="note muted"><Icon name="info" size={14} /> Audio asset — plays the working copy.</p>
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
    </div>
  {/if}
</div>

<style>
  .preview { flex: 1; overflow: auto; min-width: 0; }
  .empty { height: 100%; display: grid; place-items: center; text-align: center; padding: 20px; }
  .empty .small { font-size: 12px; margin-top: 4px; }
  .body { padding: 16px 18px; max-width: 720px; }
  .head { display: flex; align-items: center; gap: 11px; margin-bottom: 16px; }
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
  .thumb.img { padding: 0; overflow: hidden; background: repeating-conic-gradient(#2b2f35 0% 25%, #333a44 0% 50%) 50% / 24px 24px; }
  .thumb.img img { width: 100%; height: 100%; object-fit: contain; display: block; }
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
</style>
