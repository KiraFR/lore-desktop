<script lang="ts">
  import { toasts, dismissToast } from './toast'
  import Icon from './Icon.svelte'
</script>

<div class="toaster">
  {#each $toasts as t (t.id)}
    <div class="toast {t.variant}" role="alert">
      <span class="ico"><Icon name={t.variant === 'info' ? 'info' : 'alert'} size={16} /></span>
      <div class="body">
        <strong class="title">{t.title}</strong>
        {#if t.message}<p class="msg">{t.message}</p>{/if}
        {#if t.action}
          <button class="act" onclick={() => { t.action!.run(); dismissToast(t.id) }}>{t.action.label}</button>
        {/if}
      </div>
      <button class="close" onclick={() => dismissToast(t.id)} aria-label="Dismiss">×</button>
    </div>
  {/each}
</div>

<style>
  .toaster { position: fixed; right: 16px; bottom: 16px; z-index: 100; display: flex; flex-direction: column; gap: 8px; max-width: 380px; }
  .toast { display: flex; gap: 10px; align-items: flex-start; padding: 11px 12px; border-radius: var(--radius); background: var(--panel); border: 1px solid var(--border); border-left: 3px solid var(--border); box-shadow: 0 6px 20px rgba(0, 0, 0, .35); }
  .toast.error { border-color: var(--deleted); border-left-color: var(--deleted); }
  .toast.error .ico { color: var(--deleted); }
  .toast.info { border-color: var(--accent); border-left-color: var(--accent); }
  .toast.info .ico { color: var(--accent-text); }
  .ico { margin-top: 1px; }
  .body { min-width: 0; flex: 1; }
  .title { display: block; font-size: 13px; font-weight: 600; color: var(--text); }
  .msg { margin: 2px 0 0; font-size: 12px; color: var(--text-muted); word-break: break-word; }
  .act { margin-top: 8px; padding: 4px 12px; font-size: 12px; background: var(--accent); color: var(--on-accent); border: none; border-radius: 6px; }
  .close { background: none; border: none; color: var(--text-muted); font-size: 16px; line-height: 1; padding: 0 2px; }
  .close:hover { background: none; color: var(--text); }
</style>
