<script lang="ts">
  import type { ChangedFile, DiffLine } from './types'
  import { repo, setLock } from './repo.svelte'
  import { session } from './session.svelte'
  import { api } from './api'
  import Icon from './Icon.svelte'

  let { file }: { file: ChangedFile | null } = $props()

  let diff = $state<DiffLine[]>([])
  let diffLoading = $state(false)
  let diffError = $state(false)

  // Fetch the unified diff whenever a text file is selected.
  $effect(() => {
    const f = file
    if (!f || f.isBinary) { diff = []; diffError = false; diffLoading = false; return }
    const repoPath = session.config.currentRepo
    if (!repoPath) { diff = []; diffLoading = false; return }
    diffLoading = true
    diffError = false
    api
      .getDiff(repoPath, f.path)
      .then((d) => { if (file?.path === f.path) diff = d })
      .catch(() => { if (file?.path === f.path) { diffError = true; diff = [] } })
      .finally(() => { if (file?.path === f.path) diffLoading = false })
  })

  const KB = 1024, MB = 1024 * 1024
  function fmtSize(n: number): string {
    if (n >= MB) return (n / MB).toFixed(1) + ' MB'
    if (n >= KB) return (n / KB).toFixed(1) + ' KB'
    return n + ' B'
  }
  const ext = (p: string) => { const i = p.lastIndexOf('.'); return i < 0 ? '' : p.slice(i + 1).toLowerCase() }
  const TYPES: Record<string, string> = {
    uasset: 'Unreal asset', umap: 'Level (map)', cpp: 'C++ source', h: 'C++ header',
    ini: 'Config', md: 'Markdown', png: 'Texture', wav: 'Audio', fbx: 'Mesh',
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
      </header>

      {#if file.isBinary}
        <div class="cmp">
          {#if file.action !== 'add'}
            <figure class="cbox">
              <div class="thumb before"><Icon name="image" size={26} /></div>
              <figcaption>Before · previous revision</figcaption>
            </figure>
          {/if}
          {#if file.action !== 'delete'}
            <figure class="cbox">
              <div class="thumb after"><Icon name="image" size={26} /></div>
              <figcaption class="aft">{file.action === 'add' ? 'New file' : 'After · working copy'}</figcaption>
            </figure>
          {/if}
        </div>
        <p class="note muted"><Icon name="info" size={14} /> Binary asset — visual compare, no text diff.</p>
      {:else if diffLoading}
        <div class="textnote muted"><Icon name="file" size={22} /><p>Loading diff…</p></div>
      {:else if diffError}
        <div class="textnote muted"><Icon name="file" size={22} /><p>Couldn't load diff.</p></div>
      {:else if diff.length === 0}
        <div class="textnote muted"><Icon name="file" size={22} /><p>No text changes to show.</p></div>
      {:else}
        <div class="diff">
          {#each diff as line, i (i)}
            <div class="dl {line.kind}">{line.text}</div>
          {/each}
        </div>
      {/if}

      <dl class="meta">
        <div><dt>Type</dt><dd>{typeName(file.path)}</dd></div>
        <div><dt>Size</dt><dd>{sizeText}</dd></div>
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
  .cmp { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; }
  figure { margin: 0; }
  .thumb { height: 150px; border-radius: 8px; display: grid; place-items: center; color: var(--text-dim); border: 1px solid var(--border); }
  .thumb.before { background: #2b2f35; }
  .thumb.after { background: #33475f; }
  figcaption { font-size: 11px; color: var(--text-muted); margin-top: 7px; text-align: center; }
  figcaption.aft { color: var(--accent-text); }
  .note { display: flex; align-items: center; gap: 7px; font-size: 11px; margin: 12px 0 4px; }
  .textnote { display: flex; align-items: center; gap: 12px; padding: 22px; border: 1px dashed var(--border); border-radius: 8px; font-size: 12.5px; }
  .textnote p { margin: 0; }
  .diff { font-family: var(--font-mono); font-size: 12px; line-height: 1.5; border: 1px solid var(--border); border-radius: 8px; overflow-x: auto; margin: 4px 0; }
  .dl { white-space: pre; padding: 0 10px; }
  .dl.add { background: rgba(63, 185, 80, .12); color: var(--added); }
  .dl.del { background: rgba(248, 81, 73, .12); color: var(--deleted); }
  .dl.context { color: var(--text-muted); }
  .dl.hunk { color: var(--accent-text); background: var(--panel); }
  .meta { margin: 18px 0 0; }
  .meta > div { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 9px 0; border-top: 1px solid var(--border); font-size: 12.5px; }
  dt { color: var(--text-muted); }
  dd { margin: 0; display: inline-flex; align-items: center; gap: 10px; }
  .lockrow { display: inline-flex; align-items: center; gap: 6px; color: var(--accent-text); }
  .lockrow.other { color: var(--text-muted); }
  .mini { padding: 3px 10px; font-size: 11px; }
</style>
