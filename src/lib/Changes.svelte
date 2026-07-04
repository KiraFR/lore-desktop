<script lang="ts">
  import { repo, commit } from './repo.svelte'

  let message = $state('')
  let tab = $state<'changes' | 'history'>('changes')

  const glyph: Record<string, { c: string; v: string }> = {
    add: { c: 'added', v: '+' }, modify: { c: 'modified', v: '~' }, delete: { c: 'deleted', v: '−' },
    move: { c: 'modified', v: 'R' }, copy: { c: 'modified', v: 'C' },
  }
  const dir = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }
  const base = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }

  const files = $derived(repo.status?.files ?? [])
  const branch = $derived(repo.status?.branch ?? 'main')

  async function doCommit() { await commit(message); message = '' }
</script>

<section class="changes">
  <div class="tabs">
    <button class="tab" class:active={tab === 'changes'} onclick={() => (tab = 'changes')}>Changes <span class="n">{files.length}</span></button>
    <button class="tab" class:active={tab === 'history'} onclick={() => (tab = 'history')}>History</button>
  </div>

  {#if repo.error}<p class="error pad">{repo.error}</p>{/if}

  {#if tab === 'history'}
    <div class="empty muted"><p>History arrives in a later update.</p></div>
  {:else}
    <div class="filelist">
      {#if repo.busy === 'status' && !repo.status}
        <p class="muted pad">Scanning…</p>
      {:else if files.length === 0}
        <div class="empty muted"><p>No local changes.</p></div>
      {:else}
        <ul>
          {#each files as f (f.path)}
            <li class="file">
              <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>
              <span class="path"><span class="dir">{dir(f.path)}</span>{base(f.path)}</span>
              {#if f.isBinary}<span class="bin">bin</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="composer">
      <input bind:value={message} placeholder="Summary (required)" disabled={!!repo.busy} />
      <textarea rows="2" placeholder="Description" disabled={!!repo.busy}></textarea>
      <button class="accent" onclick={doCommit} disabled={!!repo.busy || !message.trim() || files.length === 0}>
        {repo.busy === 'commit' ? 'Committing…' : `Commit to ${branch}`}
      </button>
    </div>
  {/if}
</section>

<style>
  .changes { display: flex; flex-direction: column; width: 340px; flex-shrink: 0; overflow: hidden; border-right: 1px solid var(--border); }
  .tabs { display: flex; border-bottom: 1px solid var(--border); }
  .tab { flex: 1; border: none; border-radius: 0; background: none; color: var(--text-muted); padding: 9px; font-size: 13px; }
  .tab:hover { background: var(--panel); color: var(--text); }
  .tab.active { color: var(--text); box-shadow: inset 0 -2px 0 var(--accent); }
  .tab .n { color: var(--text-dim); font-size: 12px; }
  .pad { padding: 8px 12px; }
  .filelist { flex: 1; overflow: auto; }
  .filelist ul { list-style: none; margin: 0; padding: 4px 0; }
  .file { display: flex; align-items: center; gap: 8px; padding: 5px 12px; font-size: 12.5px; }
  .file:hover { background: var(--panel); }
  .tag { width: 1.1em; text-align: center; font-weight: 500; }
  .tag.added { color: var(--added); } .tag.modified { color: var(--modified); } .tag.deleted { color: var(--deleted); }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dir { color: var(--text-muted); }
  .bin { font-size: 10px; padding: 1px 5px; border: 1px solid var(--border); border-radius: 999px; color: var(--text-muted); }
  .empty { flex: 1; display: grid; place-items: center; }
  .composer { display: flex; flex-direction: column; gap: 8px; padding: 10px; border-top: 1px solid var(--border); background: var(--bg-elev); }
  .composer textarea { resize: none; }
</style>
