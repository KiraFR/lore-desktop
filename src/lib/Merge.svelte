<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import type { Branch, MergePreview, MergeConflict } from './types'
  import Icon from './Icon.svelte'

  let { onclose }: { onclose: () => void } = $props()

  let branches = $state<Branch[]>([])
  let source = $state('feature/loot')
  let target = $state('main')
  let preview = $state<MergePreview | null>(null)
  let phase = $state<'setup' | 'resolving' | 'done'>('setup')
  let conflicts = $state<MergeConflict[]>([])
  let selectedPath = $state<string | null>(null)

  const base = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
  const dir = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }
  const who = (n: string) => (n === 'you' ? 'you' : n)

  async function loadBranches() {
    const p = session.config.currentRepo
    if (!p) return
    branches = await api.getBranches(p)
    const cur = branches.find((b) => b.current)
    if (cur) target = cur.name
  }
  $effect(() => { loadBranches() })

  // Keep source ≠ target.
  $effect(() => {
    if (source === target) {
      const alt = branches.find((b) => b.name !== target)
      if (alt) source = alt.name
    }
  })

  async function loadPreview() {
    const p = session.config.currentRepo
    if (!p) return
    preview = await api.previewMerge(p, source, target)
  }
  $effect(() => {
    source; target
    if (phase === 'setup') loadPreview()
  })

  const selected = $derived(conflicts.find((c) => c.path === selectedPath) ?? null)
  const resolvedCount = $derived(conflicts.filter((c) => c.resolved).length)
  const allResolved = $derived(conflicts.length > 0 && resolvedCount === conflicts.length)

  function startMerge() {
    if (!preview || preview.commits === 0) return
    if (preview.conflicts.length === 0) { phase = 'done'; return }
    conflicts = preview.conflicts.map((c) => ({ ...c, resolved: null }))
    selectedPath = conflicts[0].path
    phase = 'resolving'
  }

  function keep(which: 'mine' | 'theirs') {
    conflicts = conflicts.map((c) => (c.path === selectedPath ? { ...c, resolved: which } : c))
    const next = conflicts.find((c) => c.path !== selectedPath && !c.resolved)
    if (next) selectedPath = next.path
  }

  const PALETTE = [
    { bg: '#14304d', fg: '#7fb0ff' }, { bg: '#3a2b12', fg: '#e3b341' },
    { bg: '#132f22', fg: '#5fca9b' }, { bg: '#301a3d', fg: '#c79bff' },
  ]
  function avatar(name: string) {
    const initials = name === 'you' ? 'JD' : name.split(/\s+/).map((w) => w[0]).join('').slice(0, 2).toUpperCase()
    let h = 0; for (let i = 0; i < name.length; i++) h += name.charCodeAt(i)
    return { initials, ...PALETTE[h % PALETTE.length] }
  }
</script>

<div class="merge">
  {#if phase === 'setup'}
    <div class="setup">
      <h3>Merge branches</h3>
      <div class="flow">
        <label class="fld">From · source branch
          <select bind:value={source}>
            {#each branches.filter((b) => b.name !== target) as b (b.name)}<option value={b.name}>{b.name}</option>{/each}
          </select>
        </label>
        <span class="arr"><Icon name="arrowRight" size={20} /></span>
        <label class="fld">Into · target · current
          <select bind:value={target}>
            {#each branches as b (b.name)}<option value={b.name}>{b.name}</option>{/each}
          </select>
        </label>
      </div>

      {#if preview}
        {#if preview.commits === 0}
          <div class="info"><Icon name="info" size={16} /> {source} has nothing to add into {target}.</div>
        {:else}
          <div class="info"><Icon name="info" size={16} /> The {preview.commits} commits on {source} are added into {target}. Nothing on the source branch is lost.</div>
        {/if}
        <div class="metrics">
          <div class="metric"><div class="k">Commits</div><div class="v">{preview.commits}</div></div>
          <div class="metric"><div class="k">Files</div><div class="v">{preview.files}</div></div>
          <div class="metric" class:warn={preview.conflicts.length > 0}><div class="k">Conflicts</div><div class="v">{preview.conflicts.length}</div></div>
        </div>
        {#if preview.conflicts.length > 0}
          <div class="warnrow"><Icon name="alert" size={15} /> {preview.conflicts.length} binary files will conflict — you'll choose a version.</div>
        {/if}
      {/if}

      <div class="actions">
        <button onclick={onclose}>Cancel</button>
        <button class="accent" onclick={startMerge} disabled={!preview || preview.commits === 0}>
          <Icon name="merge" size={15} /> Merge into {target}
        </button>
      </div>
    </div>

  {:else if phase === 'resolving'}
    <div class="resolveview">
      <div class="warnbar"><Icon name="merge" size={15} /> Merging {source} into {target} — {conflicts.length - resolvedCount} conflicts to resolve.</div>
      <div class="two">
        <div class="clist">
          {#each conflicts as c (c.path)}
            <div class="crow" class:sel={c.path === selectedPath} role="button" tabindex="0"
                 onclick={() => (selectedPath = c.path)}
                 onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectedPath = c.path } }}>
              {#if c.resolved}<span class="ci ok"><Icon name="check" size={15} /></span>{:else}<span class="ci warn"><Icon name="alert" size={15} /></span>{/if}
              <span class="path"><span class="dir">{dir(c.path)}</span>{base(c.path)}</span>
              {#if c.resolved}<span class="chip ok">resolved</span>{:else}<span class="chip">{c.isBinary ? 'binary' : 'text'}</span>{/if}
            </div>
          {/each}
        </div>

        <div class="rpane">
          {#if selected}
            {@const am = avatar(selected.mine.author)}
            {@const at = avatar(selected.theirs.author)}
            <p class="note"><Icon name="info" size={14} /> Binary asset — can't merge line by line. Pick the version to keep.</p>
            <div class="vs">
              <div class="vcard" class:pick={selected.resolved === 'mine'}>
                <div class="vhd">Mine · {target}</div>
                <div class="vsub"><span class="ava" style="background:{am.bg};color:{am.fg}">{am.initials}</span> {who(selected.mine.author)} · #{selected.mine.rev}</div>
                <div class="vthumb before"><Icon name="image" size={24} /></div>
                <button class="keep" class:done={selected.resolved === 'mine'} onclick={() => keep('mine')}>{selected.resolved === 'mine' ? 'Kept mine' : 'Keep mine'}</button>
              </div>
              <div class="vcard" class:pick={selected.resolved === 'theirs'}>
                <div class="vhd">Theirs · {source}</div>
                <div class="vsub"><span class="ava" style="background:{at.bg};color:{at.fg}">{at.initials}</span> {who(selected.theirs.author)} · #{selected.theirs.rev}</div>
                <div class="vthumb after"><Icon name="image" size={24} /></div>
                <button class="keep" class:done={selected.resolved === 'theirs'} onclick={() => keep('theirs')}>{selected.resolved === 'theirs' ? 'Kept theirs' : 'Keep theirs'}</button>
              </div>
            </div>
            <p class="tip"><Icon name="lock" size={13} /> Tip: lock binary assets before editing to avoid this next time.</p>
          {/if}
        </div>
      </div>
      <div class="resbar">
        <span>{resolvedCount} of {conflicts.length} resolved</span>
        <span class="spacer"></span>
        <button onclick={onclose}>Abort merge</button>
        <button class="accent" disabled={!allResolved} onclick={() => (phase = 'done')}>Continue merge</button>
      </div>
    </div>

  {:else}
    <div class="doneview">
      <div class="donecard">
        <div class="checkc"><Icon name="check" size={26} /></div>
        <h3>Merged into {target}</h3>
        <p class="muted">{source} was merged{conflicts.length ? ' and all conflicts resolved' : ''}.</p>
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
  .info { display: flex; align-items: center; gap: 9px; background: var(--accent-soft); border: 1px solid #244a73; border-radius: 8px; padding: 11px 13px; font-size: 12px; color: #bcd3f2; margin: 14px 0; }
  .info :global(svg) { color: #7fb0ff; flex-shrink: 0; }
  .metrics { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
  .metric { background: var(--panel); border-radius: 8px; padding: 12px; }
  .metric.warn { background: var(--warn-bg); }
  .metric .k { font-size: 11px; color: var(--text-muted); }
  .metric.warn .k { color: var(--warn-text); }
  .metric .v { font-size: 22px; font-weight: 500; margin-top: 3px; }
  .metric.warn .v { color: var(--warn-text); }
  .warnrow { display: flex; align-items: center; gap: 8px; color: var(--warn-text); font-size: 12px; margin: 14px 0; }
  .actions { display: flex; justify-content: flex-end; margin-top: 16px; }
  .actions .accent { display: inline-flex; align-items: center; gap: 7px; }

  .resolveview { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
  .warnbar { display: flex; align-items: center; gap: 9px; background: var(--warn-bg); color: var(--warn-text); padding: 10px 16px; font-size: 12px; border-bottom: 1px solid #4a3a12; }
  .two { flex: 1; display: flex; overflow: hidden; min-height: 0; }
  .clist { width: 320px; flex-shrink: 0; overflow: auto; border-right: 1px solid var(--border); }
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
  .vhd { font-size: 12px; font-weight: 500; margin-bottom: 4px; }
  .vsub { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--text-muted); margin-bottom: 11px; }
  .ava { width: 18px; height: 18px; border-radius: 50%; display: inline-grid; place-items: center; font-size: 9px; font-weight: 500; }
  .vthumb { height: 104px; border-radius: 8px; display: grid; place-items: center; color: var(--text-dim); border: 1px solid var(--border); margin-bottom: 11px; }
  .vthumb.before { background: #2b2f35; }
  .vthumb.after { background: #33475f; }
  .keep { width: 100%; }
  .keep.done { background: var(--accent-soft); border-color: #255089; color: var(--accent-text); }
  .tip { display: flex; align-items: center; gap: 7px; color: var(--text-muted); font-size: 11px; margin-top: 14px; }
  .tip :global(svg) { color: var(--accent-text); }
  .resbar { display: flex; align-items: center; gap: 10px; padding: 11px 16px; border-top: 1px solid var(--border); font-size: 12px; }

  .doneview { flex: 1; display: grid; place-items: center; padding: 20px; }
  .donecard { text-align: center; max-width: 340px; }
  .checkc { width: 52px; height: 52px; border-radius: 50%; background: var(--accent-soft); color: var(--accent-text); display: grid; place-items: center; margin: 0 auto 14px; }
  .donecard h3 { margin: 0 0 6px; font-size: 16px; font-weight: 500; }
  .donecard p { margin: 0 0 18px; font-size: 12.5px; }
</style>
