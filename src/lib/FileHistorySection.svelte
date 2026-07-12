<script lang="ts">
  import type { FileRevision } from './types'
  import { api } from './api'
  import { session } from './session.svelte'
  import { fmtSize } from './sizeFormat'
  import { repo, locks, restoreFile } from './repo.svelte'
  import { restoreAvailability } from './restoreGuard'
  import { confirmAction } from './confirm'

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

  // Guard context shared by every revision row: the file's own lock, and the
  // repo-wide clean-tree / time-travel state (a restore's sync round-trip would
  // clobber pending changes, and Lore has no stash).
  const lockHolder = $derived(locks.list.find((l) => l.path === path)?.holder ?? null)
  const dirtyTree = $derived((repo.status?.files.length ?? 0) > 0)
  const timeTraveled = $derived((repo.status?.revisionNumber ?? 0) < (repo.status?.localRevisionNumber ?? 0))

  function baseName(p: string) {
    const i = p.lastIndexOf('/')
    return i < 0 ? p : p.slice(i + 1)
  }

  async function onRestore(rev: FileRevision) {
    const avail = restoreAvailability({ isCurrent: false, dirtyTree, timeTraveled, lockHolder })
    if (!avail.canRestore) return
    const note = avail.lock === 'teammate'
      ? ` It's locked by someone else — you'll be able to restore it locally, but not commit it until they release it.`
      : avail.lock === 'free'
        ? ` The file will be locked to you so you can commit it.`
        : ''
    const ok = await confirmAction(
      `Restore ${baseName(path)} to its version at #${rev.revisionNumber}? It becomes a pending change in Changes.${note}`,
      'Restore this version',
    )
    if (ok) restoreFile(path, rev.revision, lockHolder)
  }
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
    {#each revisions.slice(0, 30) as r, i (r.revision)}
      {@const avail = restoreAvailability({ isCurrent: i === 0, dirtyTree, timeTraveled, lockHolder })}
      <li>
        <span class="tag {glyph[r.action]?.c}">{glyph[r.action]?.v ?? '?'}</span>
        <span class="frev">#{r.revisionNumber}</span>
        <span class="fmsg" title={r.message}>{r.message}</span>
        <span class="fwho">{authorLabel(r.author)}</span>
        <span class="fwhen" title={new Date(r.whenMs).toLocaleString()}>{r.when}</span>
        <span class="fsize">{fmtSize(r.size)}</span>
        {#if i !== 0}
          <button class="restore" disabled={!avail.canRestore || !!repo.busy}
                  title={avail.reason ?? 'Restore this version as a pending change'}
                  onclick={() => onRestore(r)}>Restore</button>
        {/if}
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
  .restore { flex: none; padding: 2px 8px; font-size: 10.5px; color: var(--text-muted); background: var(--bg); border: 1px solid var(--border); border-radius: 5px; }
  .restore:hover:not(:disabled) { color: var(--text); background: var(--panel-hover); }
</style>
