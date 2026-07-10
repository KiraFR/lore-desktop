<script lang="ts">
  import { repo, locks } from './repo.svelte'
  import { ui, setView } from './ui.svelte'
  import { partitionByLock } from './changesPartition'
  import Icon from './Icon.svelte'

  let collapsed = $state(false)

  const changed = $derived(partitionByLock(repo.status?.files ?? []).committable.length)
  const lockCount = $derived(locks.list.length)

  const items = [
    { id: 'changes', label: 'Changes', icon: 'edit' },
    { id: 'history', label: 'History', icon: 'history' },
    { id: 'locks', label: 'Locks', icon: 'lock' },
  ] as const
</script>

<nav class="rail" class:collapsed>
  {#each items as it (it.id)}
    <button class="item" class:active={ui.view === it.id} onclick={() => setView(it.id)} title={it.label}>
      <Icon name={it.icon} size={18} />
      {#if !collapsed}<span class="rlabel">{it.label}</span>{/if}
      {#if !collapsed && it.id === 'changes' && changed > 0}<span class="count">{changed}</span>{/if}
      {#if !collapsed && it.id === 'locks' && lockCount > 0}<span class="count">{lockCount}</span>{/if}
    </button>
  {/each}
  <button class="railtoggle" onclick={() => (collapsed = !collapsed)} title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}>
    <Icon name={collapsed ? 'chevronRight' : 'chevronLeft'} size={16} />
    {#if !collapsed}<span class="rlabel">Collapse</span>{/if}
  </button>
</nav>

<style>
  .rail { width: 168px; flex-shrink: 0; display: flex; flex-direction: column; background: var(--bg-elev); border-right: 1px solid var(--border); padding: 8px 0; transition: width .14s ease; }
  .rail.collapsed { width: 54px; }
  .item, .railtoggle { display: flex; align-items: center; gap: 11px; width: 100%; height: 38px; padding: 0 16px; background: transparent; border: none; border-radius: 0; box-shadow: none; outline: none; color: var(--text-muted); font-size: 13px; text-align: left; }
  .item:hover, .railtoggle:hover { background: var(--panel-hover); border: none; color: var(--text); }
  .item.active { background: var(--accent-soft); color: var(--accent-text); }
  .rlabel { white-space: nowrap; overflow: hidden; }
  .count { margin-left: auto; font-size: 11px; background: rgba(255, 255, 255, .09); color: var(--text-muted); border-radius: 999px; padding: 0 6px; }
  .item.active .count { background: var(--accent-ring); color: var(--accent-text); }
  .railtoggle { margin-top: auto; color: var(--text-dim); }
  .rail.collapsed .item, .rail.collapsed .railtoggle { justify-content: center; padding: 0; gap: 0; }
</style>
