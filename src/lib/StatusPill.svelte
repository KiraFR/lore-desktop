<script lang="ts">
  let { ahead = 0, behind = 0 }: { ahead?: number; behind?: number } = $props()
</script>

<span class="pill" title="{ahead} ahead / {behind} behind">
  <span class:dim={ahead === 0}>↑{ahead}</span>
  <span class:dim={behind === 0}>↓{behind}</span>
</span>

<style>
  .pill { display: inline-flex; gap: 6px; font-size: 12px; padding: 2px 8px; border: 1px solid var(--border); border-radius: 999px; }
  .dim { opacity: .4; }
</style>
