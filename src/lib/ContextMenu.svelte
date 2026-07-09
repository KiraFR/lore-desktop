<script lang="ts">
  import Icon from './Icon.svelte'

  let { x, y, items, onclose }: {
    x: number
    y: number
    items: { label: string; icon?: string; danger?: boolean; run: () => void }[]
    onclose: () => void
  } = $props()

  // Clamp so the menu never leaves the viewport: estimated size (items are a
  // fixed height) flips the opening direction near the right/bottom edges.
  const ITEM_H = 31, PAD = 12, WIDTH = 210
  const estH = $derived(items.length * ITEM_H + PAD)
  const left = $derived(x + WIDTH > window.innerWidth ? Math.max(4, x - WIDTH) : x)
  const top = $derived(y + estH > window.innerHeight ? Math.max(4, y - estH) : y)

  let menuEl = $state<HTMLDivElement>()

  // Close on pointerdown outside the menu or on Escape.
  $effect(() => {
    function onDoc(e: PointerEvent) {
      if (menuEl && !menuEl.contains(e.target as Node)) onclose()
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onclose()
    }
    document.addEventListener('pointerdown', onDoc)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('pointerdown', onDoc)
      document.removeEventListener('keydown', onKey)
    }
  })
</script>

<div class="menu" bind:this={menuEl} style="left:{left}px; top:{top}px" role="menu">
  {#each items as it (it.label)}
    <button class="action" class:danger={it.danger} role="menuitem"
            onclick={() => { it.run(); onclose() }}>
      {#if it.icon}<Icon name={it.icon} size={15} />{:else}<span class="noicon"></span>{/if}
      {it.label}
    </button>
  {/each}
</div>

<style>
  .menu { position: fixed; min-width: 210px; background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); z-index: 80; overflow: hidden; padding: 6px 0; }
  .action { display: flex; align-items: center; gap: 9px; width: 100%; padding: 7px 12px; background: transparent; border: none; border-radius: 0; box-shadow: none; color: var(--text); font-size: 12.5px; text-align: left; }
  .action:hover { background: var(--panel-hover); border: none; }
  .action :global(svg) { color: var(--text-muted); }
  .action.danger { color: var(--deleted); }
  .action.danger :global(svg) { color: var(--deleted); }
  .noicon { width: 15px; flex-shrink: 0; }
</style>
