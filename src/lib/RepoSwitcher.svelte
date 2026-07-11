<script lang="ts">
  import { api } from './api'
  import { session, selectRepo, removeRepo, relocateRepo } from './session.svelte'
  import { addExistingRepo, cloneServerRepo } from './repoActions'
  import { filterRepos, repoName } from './repoList'
  import { missingRepoPaths } from './repoHealth.svelte'
  import { toastError, toastInfo } from './toast'
  import { opProgress } from './opProgress.svelte'
  import { pct, cloneProgressLabel, cloneInFlight } from './progress'
  import Icon from './Icon.svelte'
  import type { RepoEntry } from './types'

  let { onclose, onabout }: { onclose: () => void; onabout?: () => void } = $props()

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

  // Re-point a moved repo: pick its new folder, prove it answers, then swap the
  // path in the known list (and current, if it was open). The row un-dims once
  // it's no longer flagged missing.
  async function locate(oldPath: string) {
    if (busy) return
    const parent = await api.pickFolder()
    if (!parent) return
    busy = `locate:${oldPath}`
    try {
      // Validate identity BEFORE touching the backend registry: a moved clone's
      // local metadata makes `status` work at the new path (constat a), so a
      // failing status means this folder isn't the repo — don't re-point then.
      await api.getStatus(parent) // proof of life: the folder answers as a repo
      await api.updateRepoPath(parent) // best-effort registry hygiene (clears "stale")
      await relocateRepo(oldPath, parent)
      missingRepoPaths.delete(oldPath)
      toastInfo('Repository relocated')
    } catch (e) {
      toastError("That folder doesn't answer as this repository", e)
    } finally {
      busy = ''
    }
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
        {@const missing = missingRepoPaths.has(path)}
        <div class="rowwrap" class:missing>
          <button class="item" class:cur={path === session.config.currentRepo}
                  onclick={() => switchTo(path)} disabled={!!busy || missing}
                  title={missing ? 'This folder is missing — use Locate' : undefined}>
            <Icon name="folder" size={15} />
            <span class="meta">
              <span class="rn">{repoName(path)}</span>
              <span class="rp">{path}</span>
            </span>
            {#if missing}<span class="badge">Missing</span>
            {:else if path === session.config.currentRepo}<Icon name="check" size={14} />{/if}
          </button>
          {#if missing}
            <button class="locate" title="Point this repository at its new folder"
                    onclick={() => locate(path)} disabled={!!busy}>
              {busy === `locate:${path}` ? 'Locating…' : 'Locate…'}
            </button>
          {:else}
            <button class="rm" title="Remove from list (files stay on disk)"
                    onclick={() => removeRepo(path)}>×</button>
          {/if}
        </div>
      {/each}
    </div>
    {#if session.config.currentRepo}
      <div class="div"></div>
      <button class="action" onclick={() => onabout?.()}>
        <Icon name="info" size={15} /> About repository
      </button>
    {/if}
  {:else}
    <div class="head">
      <button class="add" onclick={() => (mode = 'list')}><Icon name="chevronLeft" size={12} /> Back</button>
    </div>
    <div class="sec">Clone from {session.config.serverUrl}</div>
    {#if loading}<p class="empty">Loading repositories…</p>{/if}
    <div class="list">
      {#each serverRepos as r (r.id)}
        <div class="rowwrap">
          <button class="item" onclick={() => onClone(r)} disabled={!!busy || cloneInFlight(opProgress.clone)}>
            <Icon name="folder" size={15} />
            <span class="meta">
              <span class="rn">{r.name}</span>
              <span class="rp">{busy === `clone:${r.id}` ? cloneProgressLabel(opProgress.clone) : r.id.slice(0, 12) + '…'}</span>
            </span>
          </button>
          {#if busy === `clone:${r.id}`}
            {@const p = pct(opProgress.clone)}
            <span class="clonebar" class:indet={p === null} style="width: {p ?? 40}%"
                  role={p === null ? undefined : 'progressbar'} aria-valuemin={p === null ? undefined : 0}
                  aria-valuemax={p === null ? undefined : 100} aria-valuenow={p === null ? undefined : p}
                  aria-hidden={p === null ? 'true' : undefined}></span>
          {/if}
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
  .rowwrap { position: relative; overflow: hidden; }
  .clonebar { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--accent); transition: width .25s ease; }
  .clonebar.indet { animation: switcherslide 1.1s linear infinite; }
  @keyframes switcherslide { from { transform: translateX(-100%); } to { transform: translateX(350%); } }
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
  .rowwrap.missing .item { opacity: .6; }
  .badge { font-size: 9.5px; text-transform: uppercase; letter-spacing: .04em; padding: 1px 5px; border-radius: 4px; background: var(--warn-bg); color: var(--warn-text); }
  .locate { position: absolute; top: 50%; right: 8px; transform: translateY(-50%); padding: 3px 8px; font-size: 11px; color: var(--warn-text); background: var(--panel); border: 1px solid var(--border); border-radius: 5px; }
  .locate:hover:not(:disabled) { background: var(--panel-hover); }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 8px 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover { background: var(--panel-hover); border: none; }
  .action :global(svg) { color: var(--text-muted); }
  .div { height: 1px; background: var(--border); margin: 6px 0; }
</style>
