<script lang="ts">
  import type { PreviewData } from './types'
  import { api } from './api'
  import { session } from './session.svelte'
  import { isPreviewableImage } from './previewKind'
  import Icon from './Icon.svelte'
  import AudioPlayer from './AudioPlayer.svelte'
  import ModelViewer from './ModelViewer.svelte'

  // Working-copy media preview (image thumbnail / audio / 3D) of one repo
  // file. Shared by FilePreview (Changes, compare mode) and HistoryFilePreview
  // (single-box mode). `preview` is bindable so parents can read dimensions.
  let { path, action, compare = true, preview = $bindable(null) }: {
    path: string
    action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
    /** true = before/after compare boxes with the Changes captions and notes;
     *  false = a single working-copy box (the History panel has its own caveat). */
    compare?: boolean
    preview?: PreviewData | null
  } = $props()

  let lastPath = ''

  // Same anti-race pattern as the other lazy fetches: check the selection
  // still matches on arrival. Deleted files have no working copy to preview.
  $effect(() => {
    const p = path
    const repoPath = session.config.currentRepo
    if (action === 'delete' || !repoPath) { preview = null; lastPath = ''; return }
    const same = p === lastPath
    lastPath = p
    if (!same) preview = null
    api.getPreview(repoPath, p)
      .then((r) => { if (path === p) preview = r })
      .catch(() => { if (path === p) preview = null })
  })

  const baseName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
</script>

{#if preview?.kind === 'audio' && preview.url}
  <AudioPlayer src={preview.url} name={baseName(path)} />
  {#if compare}<p class="note muted"><Icon name="info" size={14} /> Audio asset — plays the working copy.</p>{/if}
{:else if preview?.kind === 'model' && preview.url}
  <ModelViewer url={preview.url} name={baseName(path)} />
  {#if compare}<p class="note muted"><Icon name="info" size={14} /> 3D preview of the working copy — drag to orbit, scroll to zoom.</p>{/if}
{:else if compare}
  <div class="cmp">
    {#if action !== 'add'}
      <figure class="cbox">
        <div class="thumb before"><Icon name="image" size={26} /></div>
        <figcaption>Before · previous revision</figcaption>
      </figure>
    {/if}
    {#if action !== 'delete'}
      <figure class="cbox">
        {#if preview?.kind === 'image' && preview.url}
          <div class="thumb after img"><img src={preview.url} alt={baseName(path)} /></div>
        {:else}
          <div class="thumb after"><Icon name="image" size={26} /></div>
        {/if}
        <figcaption class="aft">{action === 'add' ? 'New file' : 'After · working copy'}</figcaption>
      </figure>
    {/if}
  </div>
  {#if preview?.kind === 'image'}
    <p class="note muted"><Icon name="info" size={14} /> Previous-revision preview needs server support — working copy only.</p>
  {:else}
    <p class="note muted"><Icon name="info" size={14} /> Binary asset — visual compare, no text diff.</p>
  {/if}
{:else}
  {#if preview?.kind === 'image' && preview.url}
    <div class="thumb single img"><img src={preview.url} alt={baseName(path)} /></div>
  {:else}
    <div class="thumb single"><Icon name={isPreviewableImage(path) ? 'image' : 'file'} size={26} /></div>
  {/if}
{/if}

<style>
  .cmp { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; }
  figure { margin: 0; }
  .thumb { height: 150px; border-radius: 8px; display: grid; place-items: center; color: var(--text-dim); border: 1px solid var(--border); }
  .thumb.before { background: #2b2f35; }
  .thumb.after { background: #33475f; }
  .thumb.single { height: 190px; background: #2b2f35; }
  /* The box is a grid with auto rows, where a percentage height on the img
     resolves as auto (335×335 in a 335×149 box, then clipped). Absolute
     positioning sizes the img against the box itself instead. */
  .thumb.img { padding: 0; overflow: hidden; position: relative; background: repeating-conic-gradient(#2b2f35 0% 25%, #333a44 0% 50%) 50% / 24px 24px; }
  .thumb.img img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; }
  figcaption { font-size: 11px; color: var(--text-muted); margin-top: 7px; text-align: center; }
  figcaption.aft { color: var(--accent-text); }
  .note { display: flex; align-items: center; gap: 7px; font-size: 11px; margin: 12px 0 4px; }
</style>
