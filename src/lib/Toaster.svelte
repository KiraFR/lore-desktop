<script lang="ts">
  import { toasts, dismissToast } from './toast'
  import Icon from './Icon.svelte'
</script>

<div class="toaster">
  {#each $toasts as t (t.id)}
    <div class="toast" role="alert">
      <span class="ico"><Icon name="alert" size={16} /></span>
      <div class="body">
        <strong class="title">{t.title}</strong>
        {#if t.message}<p class="msg">{t.message}</p>{/if}
      </div>
      <button class="close" onclick={() => dismissToast(t.id)} aria-label="Dismiss">×</button>
    </div>
  {/each}
</div>

<style>
  .toaster { position: fixed; right: 16px; bottom: 16px; z-index: 100; display: flex; flex-direction: column; gap: 8px; max-width: 380px; }
  .toast { display: flex; gap: 10px; align-items: flex-start; padding: 11px 12px; border-radius: var(--radius); background: var(--panel); border: 1px solid var(--deleted); border-left: 3px solid var(--deleted); box-shadow: 0 6px 20px rgba(0, 0, 0, .35); }
  .ico { color: var(--deleted); margin-top: 1px; }
  .body { min-width: 0; flex: 1; }
  .title { display: block; font-size: 13px; font-weight: 600; color: var(--text); }
  .msg { margin: 2px 0 0; font-size: 12px; color: var(--text-muted); word-break: break-word; }
  .close { background: none; border: none; color: var(--text-muted); font-size: 16px; line-height: 1; padding: 0 2px; }
  .close:hover { background: none; color: var(--text); }
</style>
