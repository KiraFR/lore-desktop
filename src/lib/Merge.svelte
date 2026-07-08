<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { refreshStatus } from './repo.svelte'
  import { toastError } from './toast'
  import type { Branch, MergePreview, MergeConflict } from './types'
  import Icon from './Icon.svelte'

  let { onclose }: { onclose: () => void } = $props()

  let branches = $state<Branch[]>([])
  let source = $state('')
  let preview = $state<MergePreview | null>(null)
  let phase = $state<'setup' | 'resolving' | 'done'>('setup')
  let conflicts = $state<MergeConflict[]>([])
  let selectedPath = $state<string | null>(null)
  // Which side the user kept per path (local UI state — the backend only reports
  // resolved/unresolved, not which version won).
  let resolvedSide = $state<Record<string, 'mine' | 'theirs'>>({})
  let busy = $state<'' | 'merge' | 'start' | 'resolve' | 'commit' | 'abort'>('')

  // The merge always targets the current branch (Lore's `branch merge <source>`).
  const target = $derived(branches.find((b) => b.current)?.name ?? 'main')
  const others = $derived(branches.filter((b) => b.name !== target))

  const base = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
  const dir = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }

  async function loadBranches() {
    const p = session.config.currentRepo
    if (!p) return
    try { branches = await api.getBranches(p) }
    catch (e) { toastError("Couldn't load branches", e) }
  }
  $effect(() => { loadBranches() })

  // Default the source to the first non-current branch; keep it valid.
  $effect(() => {
    if (others.length && !others.some((b) => b.name === source)) source = others[0].name
  })

  async function loadPreview() {
    const p = session.config.currentRepo
    if (!p || !source) { preview = null; return }
    try { preview = await api.previewMerge(p, source, target) }
    catch (e) { toastError("Couldn't preview merge", e); preview = null }
  }
  $effect(() => { source; target; if (phase === 'setup') loadPreview() })

  const canMerge = $derived(!!preview && preview.files > 0 && preview.conflicts === 0)
  const unresolvedCount = $derived(conflicts.filter((c) => c.unresolved).length)
  const selected = $derived(conflicts.find((c) => c.path === selectedPath) ?? null)

  // Clean merge → auto-commit and done.
  async function doMerge() {
    const p = session.config.currentRepo
    if (!p || !canMerge || busy) return
    busy = 'merge'
    try {
      await api.mergeBranch(p, source, `Merge ${source} into ${target}`)
      await refreshStatus()
      phase = 'done'
    } catch (e) { toastError('Merge failed', e) }
    finally { busy = '' }
  }

  // Conflicting merge → enter the resolution phase.
  async function startMerge() {
    const p = session.config.currentRepo
    if (!p || busy) return
    busy = 'start'
    try {
      await api.mergeStart(p, source)
      conflicts = await api.mergeConflicts(p)
      resolvedSide = {}
      selectedPath = conflicts[0]?.path ?? null
      phase = 'resolving'
    } catch (e) { toastError('Merge failed', e) }
    finally { busy = '' }
  }

  async function resolve(path: string, side: 'mine' | 'theirs') {
    const p = session.config.currentRepo
    if (!p || busy) return
    busy = 'resolve'
    try {
      await api.mergeResolve(p, path, side)
      resolvedSide = { ...resolvedSide, [path]: side }
      conflicts = await api.mergeConflicts(p)
      // Auto-advance to the next still-unresolved conflict.
      const next = conflicts.find((c) => c.path !== path && c.unresolved)
      if (next) selectedPath = next.path
    } catch (e) { toastError('Resolve failed', e) }
    finally { busy = '' }
  }

  async function complete() {
    const p = session.config.currentRepo
    if (!p || unresolvedCount > 0 || busy) return
    busy = 'commit'
    try {
      await api.mergeCommit(p, `Merge ${source} into ${target}`)
      await refreshStatus()
      phase = 'done'
    } catch (e) { toastError('Merge commit failed', e) }
    finally { busy = '' }
  }

  async function abort() {
    const p = session.config.currentRepo
    if (!p || busy) return
    busy = 'abort'
    try { await api.mergeAbort(p); await refreshStatus() }
    catch (e) { toastError('Abort failed', e) }
    finally { busy = ''; onclose() }
  }
</script>

<div class="merge">
  {#if phase === 'setup'}
    <div class="setup">
      <h3>Merge a branch into {target}</h3>
      <div class="flow">
        <label class="fld">From · source branch
          <select bind:value={source}>
            {#each others as b (b.name)}<option value={b.name}>{b.name}</option>{/each}
          </select>
        </label>
        <span class="arr"><Icon name="arrowRight" size={20} /></span>
        <div class="fld">Into · current branch
          <div class="targetlbl">{target}</div>
        </div>
      </div>

      {#if preview}
        {#if preview.files === 0 && preview.conflicts === 0}
          <div class="info"><Icon name="info" size={16} /> {source} has nothing to add into {target}.</div>
        {:else}
          <div class="info"><Icon name="info" size={16} /> {preview.files} file change{preview.files === 1 ? '' : 's'} from {source} will be added into {target}.</div>
        {/if}
        <div class="metrics">
          <div class="metric"><div class="k">Files</div><div class="v">{preview.files}</div></div>
          <div class="metric" class:warn={preview.conflicts > 0}><div class="k">Conflicts</div><div class="v">{preview.conflicts}</div></div>
        </div>
        {#if preview.conflicts > 0}
          <div class="warnrow"><Icon name="alert" size={15} /> {preview.conflicts} conflicting file{preview.conflicts === 1 ? '' : 's'} — you'll choose a version for each.</div>
        {/if}
      {/if}

      <div class="actions">
        <button onclick={onclose}>Cancel</button>
        {#if preview && preview.conflicts > 0}
          <button class="accent" onclick={startMerge} disabled={busy !== ''}>
            <Icon name="merge" size={15} /> {busy === 'start' ? 'Starting…' : `Resolve & merge into ${target}`}
          </button>
        {:else}
          <button class="accent" onclick={doMerge} disabled={!canMerge || busy !== ''}>
            <Icon name="merge" size={15} /> {busy === 'merge' ? 'Merging…' : `Merge into ${target}`}
          </button>
        {/if}
      </div>
    </div>

  {:else if phase === 'resolving'}
    <div class="resolveview">
      <div class="warnbar">
        <Icon name="merge" size={15} /> Merging {source} into {target} — {unresolvedCount} of {conflicts.length} to resolve.
      </div>
      <div class="two">
        <div class="clist">
          {#each conflicts as c (c.path)}
            <div class="crow" class:sel={c.path === selectedPath} role="button" tabindex="0"
                 onclick={() => (selectedPath = c.path)}
                 onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectedPath = c.path } }}>
              {#if c.unresolved}<span class="ci warn"><Icon name="alert" size={15} /></span>{:else}<span class="ci ok"><Icon name="check" size={15} /></span>{/if}
              <span class="path"><span class="dir">{dir(c.path)}</span>{base(c.path)}</span>
              {#if c.unresolved}<span class="chip">{c.isBinary ? 'binary' : 'text'}</span>{:else}<span class="chip ok">resolved</span>{/if}
            </div>
          {/each}
        </div>

        <div class="rpane">
          {#if selected}
            <p class="note">
              <Icon name="info" size={14} />
              {selected.isBinary ? "Binary asset — can't merge line by line. Pick the version to keep." : 'Pick the version to keep for this file.'}
            </p>
            <div class="vs">
              <div class="vcard" class:pick={resolvedSide[selected.path] === 'mine'}>
                <div class="vhd">Mine · {target}</div>
                <div class="vthumb before"><Icon name="image" size={24} /></div>
                <button class="keep" class:done={resolvedSide[selected.path] === 'mine'} disabled={busy !== ''} onclick={() => resolve(selected.path, 'mine')}>
                  {resolvedSide[selected.path] === 'mine' ? 'Kept mine' : 'Keep mine'}
                </button>
              </div>
              <div class="vcard" class:pick={resolvedSide[selected.path] === 'theirs'}>
                <div class="vhd">Theirs · {source}</div>
                <div class="vthumb after"><Icon name="image" size={24} /></div>
                <button class="keep" class:done={resolvedSide[selected.path] === 'theirs'} disabled={busy !== ''} onclick={() => resolve(selected.path, 'theirs')}>
                  {resolvedSide[selected.path] === 'theirs' ? 'Kept theirs' : 'Keep theirs'}
                </button>
              </div>
            </div>
            <p class="tip"><Icon name="lock" size={13} /> Tip: lock binary assets before editing to avoid this next time.</p>
          {/if}
        </div>
      </div>
      <div class="resbar">
        <span>{conflicts.length - unresolvedCount} of {conflicts.length} resolved</span>
        <span class="spacer"></span>
        <button onclick={abort} disabled={busy !== ''}>Abort merge</button>
        <button class="accent" disabled={unresolvedCount > 0 || busy !== ''} onclick={complete}>
          {busy === 'commit' ? 'Completing…' : 'Complete merge'}
        </button>
      </div>
    </div>

  {:else}
    <div class="doneview">
      <div class="donecard">
        <div class="checkc"><Icon name="check" size={26} /></div>
        <h3>Merged into {target}</h3>
        <p class="muted">{source} was merged into {target}.</p>
        <button class="accent" onclick={onclose}>Done</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .merge { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-width: 0; }

  .setup { padding: 24px 26px; max-width: 560px; overflow: auto; }
  .setup h3 { margin: 0 0 16px; font-size: 15px; font-weight: 500; }
  .flow { display: grid; grid-template-columns: 1fr auto 1fr; align-items: end; gap: 12px; }
  .fld { display: flex; flex-direction: column; gap: 6px; font-size: 11px; color: var(--text-muted); }
  .arr { color: var(--text-muted); padding-bottom: 8px; }
  select { width: 100%; padding: 8px 10px; border-radius: var(--radius); border: 1px solid var(--border-strong); background: var(--panel); color: var(--text); font: inherit; }
  .targetlbl { padding: 8px 10px; border-radius: var(--radius); border: 1px solid var(--border); background: var(--bg); color: var(--text); font-size: 13px; }
  .info { display: flex; align-items: center; gap: 9px; background: var(--accent-soft); border: 1px solid #244a73; border-radius: 8px; padding: 11px 13px; font-size: 12px; color: #bcd3f2; margin: 14px 0; }
  .info :global(svg) { color: #7fb0ff; flex-shrink: 0; }
  .metrics { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; }
  .metric { background: var(--panel); border-radius: 8px; padding: 12px; }
  .metric.warn { background: var(--warn-bg); }
  .metric .k { font-size: 11px; color: var(--text-muted); }
  .metric.warn .k { color: var(--warn-text); }
  .metric .v { font-size: 22px; font-weight: 500; margin-top: 3px; }
  .metric.warn .v { color: var(--warn-text); }
  .warnrow { display: flex; align-items: center; gap: 8px; color: var(--warn-text); font-size: 12px; margin: 14px 0; }
  .warnrow :global(svg) { flex-shrink: 0; }
  .actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }
  .actions .accent { display: inline-flex; align-items: center; gap: 7px; }

  .resolveview { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
  .warnbar { display: flex; align-items: center; gap: 9px; background: var(--warn-bg); color: var(--warn-text); padding: 10px 16px; font-size: 12px; border-bottom: 1px solid #4a3a12; }
  .two { flex: 1; display: flex; overflow: hidden; min-height: 0; }
  .clist { width: 300px; flex-shrink: 0; overflow: auto; border-right: 1px solid var(--border); }
  .crow { display: flex; align-items: center; gap: 9px; padding: 11px 14px; border-bottom: 1px solid var(--border); font-size: 12.5px; cursor: pointer; }
  .crow:hover { background: var(--panel); }
  .crow.sel { background: var(--accent-soft); }
  .ci { display: inline-flex; flex-shrink: 0; }
  .ci.ok { color: var(--added); }
  .ci.warn { color: var(--warn-text); }
  .chip { margin-left: auto; flex-shrink: 0; font-size: 10px; color: var(--text-muted); border: 1px solid var(--border); border-radius: 999px; padding: 1px 7px; }
  .chip.ok { color: var(--added); border-color: #245029; }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .dir { color: var(--text-muted); }
  .rpane { flex: 1; overflow: auto; padding: 16px 18px; min-width: 0; }
  .note { display: flex; align-items: center; gap: 7px; color: var(--text-muted); font-size: 11px; margin: 0 0 12px; }
  .vs { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 14px; }
  .vcard { border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 13px; }
  .vcard.pick { border-color: var(--accent); }
  .vhd { font-size: 12px; font-weight: 500; margin-bottom: 11px; }
  .vthumb { height: 104px; border-radius: 8px; display: grid; place-items: center; color: var(--text-dim); border: 1px solid var(--border); margin-bottom: 11px; }
  .vthumb.before { background: #2b2f35; }
  .vthumb.after { background: #33475f; }
  .keep { width: 100%; }
  .keep.done { background: var(--accent-soft); border-color: #255089; color: var(--accent-text); }
  .tip { display: flex; align-items: center; gap: 7px; color: var(--text-muted); font-size: 11px; margin-top: 14px; }
  .tip :global(svg) { color: var(--accent-text); }
  .resbar { display: flex; align-items: center; gap: 10px; padding: 11px 16px; border-top: 1px solid var(--border); font-size: 12px; }
  .spacer { flex: 1; }

  .doneview { flex: 1; display: grid; place-items: center; padding: 20px; }
  .donecard { text-align: center; max-width: 340px; }
  .checkc { width: 52px; height: 52px; border-radius: 50%; background: var(--accent-soft); color: var(--accent-text); display: grid; place-items: center; margin: 0 auto 14px; }
  .donecard h3 { margin: 0 0 6px; font-size: 16px; font-weight: 500; }
  .donecard p { margin: 0 0 18px; font-size: 12.5px; }
</style>
