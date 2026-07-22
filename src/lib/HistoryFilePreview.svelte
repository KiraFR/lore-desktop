<script lang="ts">
  import type { CommitFile, DiffLine, FileRevision, PreviewData } from './types'
  import Icon from './Icon.svelte'
  import MediaPreview from './MediaPreview.svelte'
  import DiffBlock from './DiffBlock.svelte'
  import FileHistorySection from './FileHistorySection.svelte'
  import { typeName, isTextDiffable } from './fileTypes'
  import { fmtSize } from './sizeFormat'
  import { api } from './api'
  import { session } from './session.svelte'

  // Lightweight preview panel for a file of a History commit: working-copy
  // media + type/size + the file's revision timeline. Deliberately NO Discard
  // and NO Lock — those act on the working copy, out of place next to an
  // arbitrary commit.
  let { file, isTip, revision, parent, onclose }: {
    file: CommitFile
    /** The selected commit is the local tip — the disk matches it, no caveat needed. */
    isTip: boolean
    /** The selected commit's signature — the diff "target". */
    revision: string
    /** The selected commit's first parent signature — the diff "source". */
    parent: string
    onclose: () => void
  } = $props()

  const baseName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
  const dirName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }

  const badge = $derived(
    file.action === 'add' ? { t: 'Added', c: 'added' } :
    file.action === 'delete' ? { t: 'Deleted', c: 'deleted' } :
    { t: 'Modified', c: 'modified' },
  )

  let preview = $state<PreviewData | null>(null)
  let revisions = $state<FileRevision[]>([])
  // Best available size without `file cat <rev>`: the newest committed revision.
  const sizeText = $derived(revisions[0] ? fmtSize(revisions[0].size) : '—')

  const textDiffable = $derived(isTextDiffable(file.path))

  let diffLines = $state<DiffLine[] | null>(null)
  let diffLoading = $state(false)
  let lastDiffPath = ''

  // Same anti-race pattern as MediaPreview: capture the path, check it still
  // matches on arrival. Only fetched for text files — the historical diff is
  // accurate for this commit regardless of `isTip`.
  $effect(() => {
    const p = file.path
    const repoPath = session.config.currentRepo
    if (!isTextDiffable(p) || !repoPath) { diffLines = null; diffLoading = false; lastDiffPath = ''; return }
    const same = p === lastDiffPath
    lastDiffPath = p
    if (!same) diffLines = null
    diffLoading = true
    api.getFileDiffAt(repoPath, p, parent, revision)
      .then((lines) => { if (file.path === p) diffLines = lines })
      .catch(() => { if (file.path === p) diffLines = null })
      .finally(() => { if (file.path === p) diffLoading = false })
  })
</script>

<div class="hpreview">
  <div class="body">
    <header class="head">
      <div class="ic"><Icon name="file" size={20} /></div>
      <div class="ttl">
        <div class="fn">{baseName(file.path)}</div>
        <div class="fp muted">{dirName(file.path)}</div>
      </div>
      <span class="badge {badge.c}">{badge.t}</span>
      <button class="close" onclick={onclose} title="Close preview (Esc)" aria-label="Close preview">×</button>
    </header>

    {#if textDiffable}
      <div class="diffhead">Changes in this revision</div>
      {#if diffLoading}
        <p class="muted">Loading diff…</p>
      {:else if diffLines && diffLines.length}
        <DiffBlock lines={diffLines} />
      {:else}
        <p class="muted">No textual changes.</p>
      {/if}
    {:else if file.action === 'delete'}
      <div class="gone">
        <Icon name="file" size={26} />
        <p>No longer in the working copy</p>
      </div>
    {:else}
      {#if !isTip}
        <p class="wcnote" role="note"><Icon name="info" size={14} /> Preview of the current working copy — this commit's version can't be shown yet.</p>
      {/if}
      <MediaPreview path={file.path} action={file.action} compare={false} bind:preview />
    {/if}

    <dl class="meta">
      <div><dt>Type</dt><dd>{typeName(file.path)}</dd></div>
      <div><dt>Size</dt><dd>{sizeText}</dd></div>
      {#if file.action !== 'delete' && preview?.width && preview?.height}
        <div><dt>Dimensions</dt><dd>{preview.width} × {preview.height}</dd></div>
      {/if}
    </dl>

    <FileHistorySection path={file.path} bind:revisions />
  </div>
</div>

<style>
  .hpreview { flex: 1; overflow: auto; min-width: 0; border-left: 1px solid var(--border); }
  .body { padding: 16px 18px; }
  .head { display: flex; align-items: center; gap: 11px; margin-bottom: 16px; }
  .ic { width: 34px; height: 34px; border-radius: 8px; background: var(--panel); display: grid; place-items: center; color: var(--text-muted); flex-shrink: 0; }
  .ttl { min-width: 0; flex: 1; }
  .fn { font-size: 14px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fp { font-size: 11px; }
  .badge { border-radius: var(--radius); padding: 3px 9px; font-size: 11px; flex-shrink: 0; }
  .badge.modified { background: var(--warn-bg); color: var(--warn-text); }
  .badge.added { background: rgba(63, 185, 80, .15); color: var(--added); }
  .badge.deleted { background: rgba(248, 81, 73, .15); color: var(--deleted); }
  .close { width: 24px; height: 24px; padding: 0; line-height: 1; font-size: 15px; color: var(--text-muted); flex-shrink: 0; }
  .diffhead { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: .04em; margin: 0 0 8px; }
  .wcnote { display: flex; align-items: center; gap: 7px; font-size: 11px; color: var(--text-muted); margin: 0 0 10px; }
  .gone { display: flex; align-items: center; gap: 12px; padding: 22px; border: 1px dashed var(--border); border-radius: 8px; color: var(--text-muted); font-size: 12.5px; }
  .gone p { margin: 0; }
  .meta { margin: 18px 0 0; }
  .meta > div { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 9px 0; border-top: 1px solid var(--border); font-size: 12.5px; }
  dt { color: var(--text-muted); }
  dd { margin: 0; display: inline-flex; align-items: center; gap: 10px; }
</style>
