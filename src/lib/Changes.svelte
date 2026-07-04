<script lang="ts">
  import { api } from './api'
  import StatusPill from './StatusPill.svelte'
  import type { StatusResult } from './types'

  let { repoPath }: { repoPath: string } = $props()

  let status = $state<StatusResult | null>(null)
  let message = $state('')
  let busy = $state<'' | 'status' | 'commit' | 'push' | 'sync'>('')
  let error = $state('')

  const tag: Record<string, { c: string; v: string }> = {
    add: { c: 'add', v: 'A' }, modify: { c: 'modify', v: 'M' }, delete: { c: 'delete', v: 'D' },
    move: { c: 'modify', v: 'R' }, copy: { c: 'modify', v: 'C' },
  }
  const kb = (n: number) => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.ceil(n / 1024)} KB`)

  async function refresh() {
    error = ''; busy = 'status'
    try { status = await api.getStatus(repoPath) }
    catch (e) { error = String(e) } finally { busy = '' }
  }
  async function run(kind: 'commit' | 'push' | 'sync') {
    error = ''; busy = kind
    try {
      if (kind === 'commit') { await api.commitAll(repoPath, message); message = '' }
      else if (kind === 'push') await api.push(repoPath)
      else await api.sync(repoPath)
      await refresh()
    } catch (e) { error = String(e) } finally { busy = '' }
  }

  $effect(() => { repoPath; refresh() })
</script>

<section class="changes">
  <div class="toolbar">
    <span class="branch">⎇ {status?.branch ?? '…'}</span>
    <StatusPill ahead={status?.localAhead ?? 0} behind={status?.remoteAhead ?? 0} />
    <span class="spacer"></span>
    <button onclick={refresh} disabled={!!busy}>↻</button>
    <button onclick={() => run('sync')} disabled={!!busy}>{busy === 'sync' ? 'Syncing…' : 'Sync'}</button>
    <button onclick={() => run('push')} disabled={!!busy || (status?.localAhead ?? 0) === 0}>
      {busy === 'push' ? 'Pushing…' : `Push${status?.localAhead ? ` (${status.localAhead})` : ''}`}
    </button>
  </div>

  {#if error}<p class="error pad">{error}</p>{/if}

  <div class="filelist">
    {#if busy === 'status' && !status}
      <p class="muted pad">Scanning…</p>
    {:else if status && status.files.length === 0}
      <div class="empty muted">
        <div class="big">✓</div>
        <p>No local changes.</p>
      </div>
    {:else}
      <ul>
        {#each status?.files ?? [] as f (f.path)}
          <li class="file">
            <span class="tag {tag[f.action]?.c}">{tag[f.action]?.v ?? '?'}</span>
            <span class="path">{f.path}</span>
            {#if f.isBinary}<span class="binary" title="Binary file">bin</span>{/if}
            <span class="spacer"></span>
            <span class="size muted">{f.action === 'delete' ? '—' : kb(f.size)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="composer">
    <textarea bind:value={message} rows="2" placeholder="Summary of your changes" disabled={!!busy}></textarea>
    <button class="primary" onclick={() => run('commit')}
      disabled={!!busy || !message.trim() || !status?.files.length}>
      {busy === 'commit' ? 'Committing…' : `Commit ${status?.files.length ?? 0} file${(status?.files.length ?? 0) === 1 ? '' : 's'}`}
    </button>
  </div>
</section>

<style>
  .changes { display: flex; flex-direction: column; flex: 1; overflow: hidden; }
  .toolbar { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-bottom: 1px solid var(--border); }
  .branch { font-weight: 600; }
  .pad { padding: 8px 12px; }
  .filelist { flex: 1; overflow: auto; }
  .filelist ul { list-style: none; margin: 0; padding: 4px 0; }
  .file { display: flex; align-items: center; gap: 10px; padding: 5px 14px; font-family: var(--mono); font-size: 12.5px; }
  .file:hover { background: var(--bg-elev); }
  .tag { width: 1.4em; text-align: center; font-weight: 700; border-radius: 4px; }
  .tag.add { color: var(--add); } .tag.modify { color: var(--modify); } .tag.delete { color: var(--delete); }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .binary { font-size: 10px; padding: 0 5px; border: 1px solid var(--border); border-radius: 999px; color: var(--muted); }
  .size { font-size: 11px; }
  .empty { display: grid; place-items: center; height: 100%; gap: 4px; }
  .empty .big { font-size: 40px; color: var(--add); }
  .composer { display: flex; flex-direction: column; gap: 8px; padding: 12px; border-top: 1px solid var(--border); background: var(--bg-elev); }
  .composer textarea { resize: none; }
</style>
