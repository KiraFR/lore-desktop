<script lang="ts">
  import type { DiffLine } from './types'

  // Unified-diff line rendering, shared by FilePreview (full diff) and the
  // Merge conflict cards (mini-diff capped by `maxLines`; 0 = no cap).
  let { lines, maxLines = 0 }: { lines: DiffLine[]; maxLines?: number } = $props()

  const shown = $derived(maxLines > 0 ? lines.slice(0, maxLines) : lines)
</script>

<div class="diff">
  {#each shown as line, i (i)}
    <div class="dl {line.kind}">
      <span class="ln">{line.oldLine ?? ''}</span>
      <span class="ln">{line.newLine ?? ''}</span>
      <span class="mk">{line.kind === 'add' ? '+' : line.kind === 'del' ? '-' : ''}</span>
      <span class="tx">{line.text}</span>
    </div>
  {/each}
  {#if maxLines > 0 && lines.length > maxLines}
    <div class="more">… {lines.length - maxLines} more lines</div>
  {/if}
</div>

<style>
  .diff { font-family: var(--font-mono); font-size: 12px; line-height: 1.55; border: 1px solid var(--border); border-radius: 8px; overflow-x: auto; margin: 4px 0; }
  .dl { display: flex; }
  .ln { flex: 0 0 44px; text-align: right; padding: 0 8px; color: var(--text-dim); user-select: none; }
  .mk { flex: 0 0 16px; text-align: center; color: var(--text-dim); user-select: none; }
  .tx { flex: 1; white-space: pre; padding-right: 12px; }
  .dl.add { background: rgba(63, 185, 80, .12); }
  .dl.add .mk, .dl.add .tx { color: var(--added); }
  .dl.del { background: rgba(248, 81, 73, .12); }
  .dl.del .mk, .dl.del .tx { color: var(--deleted); }
  .dl.context .tx { color: var(--text-muted); }
  .dl.hunk { background: var(--panel); }
  .dl.hunk .tx { color: var(--accent-text); }
  .more { padding: 3px 12px; font-size: 11px; color: var(--text-dim); border-top: 1px solid var(--border); }
</style>
