<script lang="ts">
  import { api } from './api'
  import { session, selectRepo, removeRepo } from './session.svelte'
  import { addExistingRepo, cloneServerRepo } from './repoActions'
  import { filterRepos, repoName } from './repoList'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'
  import type { RepoEntry } from './types'

  let { onclose }: { onclose: () => void } = $props()

  let filter = $state('')
  let addOpen = $state(false)
  // 'list' = known repos; 'clone' = pick a server repo to clone.
  let mode = $state<'list' | 'clone'>('list')
  let serverRepos = $state<RepoEntry[]>([])
  let loading = $state(false)
  // '' | 'add' | `clone:<id>` — drives the in-flight labels.
  let busy = $state('')

  const shown = $derived(filterRepos(session.config.recentRepos, filter))

  async function switchTo(path: string) {
    if (busy) return
    if (path !== session.config.currentRepo) await selectRepo(path)
    onclose()
  }

  async function onAddExisting() {
    addOpen = false
    if (busy) return
    busy = 'add'
    try {
      if (await addExistingRepo()) onclose()
    } finally {
      busy = ''
    }
  }

  async function enterClone() {
    addOpen = false
    mode = 'clone'
    loading = true
    try {
      serverRepos = await api.listRepos(session.config.serverUrl!)
    } catch (e) {
      toastError("Couldn't list repositories", e)
    } finally {
      loading = false
    }
  }

  async function onClone(entry: RepoEntry) {
    if (busy) return
    busy = `clone:${entry.id}`
    try {
      if (await cloneServerRepo(entry)) onclose()
    } finally {
      busy = ''
    }
  }
</script>

<div class="menu">
  {#if mode === 'list'}
    <div class="head">
      <input class="search" bind:value={filter} placeholder="Filter repositories" />
      <div class="addzone">
        <button class="add" class:open={addOpen} onclick={() => (addOpen = !addOpen)}>
          <Icon name="plus" size={13} /> Add <Icon name="chevronDown" size={12} />
        </button>
        {#if addOpen}
          <div class="addmenu">
            <button class="action" onclick={enterClone}><Icon name="sync" size={15} /> Clone repository…</button>
            <button class="action" onclick={onAddExisting} disabled={busy === 'add'}>
              <Icon name="folder" size={15} /> {busy === 'add' ? 'Opening…' : 'Add existing repository…'}
            </button>
          </div>
        {/if}
      </div>
    </div>
    <div class="sec">Repositories · {shown.length.toLocaleString()}</div>
    {#if shown.length === 0}
      <p class="empty">{filter.trim() ? 'No repositories match' : 'No repositories yet — use Add'}</p>
    {/if}
    <div class="list">
      {#each shown as path (path)}
        <div class="rowwrap">
          <button class="item" class:cur={path === session.config.currentRepo}
                  onclick={() => switchTo(path)} disabled={!!busy}>
            <Icon name="folder" size={15} />
            <span class="meta">
              <span class="rn">{repoName(path)}</span>
              <span class="rp">{path}</span>
            </span>
            {#if path === session.config.currentRepo}<Icon name="check" size={14} />{/if}
          </button>
          <button class="rm" title="Remove from list (files stay on disk)"
                  onclick={() => removeRepo(path)}>×</button>
        </div>
      {/each}
    </div>
  {:else}
    <div class="head">
      <button class="add" onclick={() => (mode = 'list')}><Icon name="chevronLeft" size={12} /> Back</button>
    </div>
    <div class="sec">Clone from {session.config.serverUrl}</div>
    {#if loading}<p class="empty">Loading repositories…</p>{/if}
    <div class="list">
      {#each serverRepos as r (r.id)}
        <div class="rowwrap">
          <button class="item" onclick={() => onClone(r)} disabled={!!busy}>
            <Icon name="folder" size={15} />
            <span class="meta">
              <span class="rn">{r.name}</span>
              <span class="rp">{busy === `clone:${r.id}` ? 'Cloning…' : r.id.slice(0, 12) + '…'}</span>
            </span>
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu { position: absolute; top: calc(100% + 6px); left: 0; width: 320px; background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); z-index: 50; overflow: hidden; padding: 8px 0; }
  .head { display: flex; align-items: center; gap: 6px; margin: 4px 10px 8px; }
  .search { flex: 1; min-width: 0; padding: 7px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .addzone { position: relative; }
  .add { display: flex; align-items: center; gap: 4px; padding: 6px 9px; font-size: 12px; }
  .add.open { background: var(--accent-soft); border-color: var(--accent); }
  .addmenu { position: absolute; top: calc(100% + 4px); right: 0; width: 230px; background: var(--panel); border: 1px solid var(--border-strong); border-radius: 8px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); z-index: 60; overflow: hidden; padding: 4px 0; }
  .sec { font-size: 10px; text-transform: uppercase; letter-spacing: .04em; color: var(--text-dim); padding: 2px 12px 5px; }
  .empty { margin: 2px 12px 8px; font-size: 12px; color: var(--text-muted); }
  .list { max-height: 300px; overflow-y: auto; overflow-x: hidden; }
  .rowwrap { position: relative; }
  .item { display: flex; align-items: center; gap: 9px; width: 100%; padding: 7px 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .item:hover:not(:disabled) { background: var(--panel-hover); border: none; }
  .item.cur { color: var(--accent-text); }
  .item :global(svg) { color: var(--text-muted); }
  .item.cur :global(svg) { color: var(--accent-text); }
  .meta { display: flex; flex-direction: column; min-width: 0; flex: 1; line-height: 1.25; }
  .rn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rp { font-size: 10.5px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; direction: rtl; text-align: left; }
  .rm { position: absolute; top: 50%; right: 8px; transform: translateY(-50%); display: none; width: 20px; height: 20px; padding: 0; line-height: 1; font-size: 14px; color: var(--text-muted); background: var(--panel); border: 1px solid var(--border); border-radius: 5px; }
  .rowwrap:hover .rm { display: block; }
  .rm:hover { color: var(--text); background: var(--panel-hover); }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover { background: var(--panel-hover); border: none; }
  .action :global(svg) { color: var(--text-muted); }
</style>
