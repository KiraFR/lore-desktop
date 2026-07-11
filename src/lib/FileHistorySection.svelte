<script lang="ts">
  import type { FileRevision } from './types'
  import { api } from './api'
  import { session } from './session.svelte'
  import { fmtSize } from './sizeFormat'

  // Per-asset revision timeline, fetched lazily on selection (anti-race).
  // `revisions` is bindable so a parent can read the head revision (the
  // History preview derives its Size row from it).
  let { path, revisions = $bindable([]) }: { path: string; revisions?: FileRevision[] } = $props()

  let loading = $state(false)
  let error = $state(false)
  let lastPath = ''

  $effect(() => {
    const p = path
    const repoPath = session.config.currentRepo
    if (!repoPath) { revisions = []; loading = false; error = false; lastPath = ''; return }
    const same = p === lastPath
    lastPath = p
    if (!same) { revisions = []; loading = true }
    error = false
    api.getFileHistory(repoPath, p)
      .then((revs) => { if (path === p) revisions = revs })
      .catch(() => { if (path === p) error = true })
      .finally(() => { if (path === p) loading = false })
  })

  const glyph: Record<string, { c: string; v: string }> = {
    add: { c: 'added', v: '+' }, modify: { c: 'modified', v: '~' }, delete: { c: 'deleted', v: '−' },
    move: { c: 'modified', v: 'R' }, copy: { c: 'modified', v: 'C' },
  }
  const authorLabel = (a: string) =>
    a === session.identity?.email ? 'you' : a.includes('@') ? a.split('@')[0] : a.slice(0, 8)
</script>

<div class="fhhead">History{#if revisions.length} · {revisions.length} {revisions.length === 1 ? 'revision' : 'revisions'}{/if}</div>
{#if loading}
  <p class="fhnote muted">Loading history…</p>
{:else if error}
  <p class="fhnote muted">Couldn't load file history.</p>
{:else if revisions.length === 0}
  <p class="fhnote muted">No committed revisions yet.</p>
{:else}
  <ul class="fhl">
    {#each revisions.slice(0, 30) as r (r.revision)}
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
  {#if revisions.length > 30}<p class="fhnote muted">…and {revisions.length - 30} more revisions</p>{/if}
{/if}

<style>
  .fhhead { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: .04em; margin: 20px 0 6px; }
  .fhnote { font-size: 12px; margin: 4px 0; }
  .fhl { list-style: none; margin: 0; padding: 0; }
  .fhl li { display: flex; align-items: center; gap: 9px; padding: 6px 0; border-top: 1px solid var(--border); font-size: 12.5px; }
  .tag { width: 1.1em; text-align: center; font-weight: 500; flex-shrink: 0; }
  .tag.added { color: var(--added); } .tag.modified { color: var(--modified); } .tag.deleted { color: var(--deleted); }
  .frev { font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); flex: none; min-width: 28px; }
  .fmsg { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fwho { flex: none; font-size: 11px; color: var(--accent-text); }
  .fwhen { flex: none; font-size: 11px; color: var(--text-dim); }
  .fsize { flex: none; font-size: 11px; color: var(--text-muted); font-family: var(--font-mono); min-width: 58px; text-align: right; }
</style>
