<script lang="ts">
  import { api } from './api'
  import { setSignedIn } from './session.svelte'

  let serverUrl = $state('lore://lore.example.com:41337')
  let authOverride = $state('')
  let showAdvanced = $state(false)
  let busy = $state(false)
  let error = $state('')

  async function go() {
    error = ''
    if (!serverUrl.startsWith('lore://') || serverUrl.length < 9) {
      error = 'Enter a Lore server URL like lore://host:41337'; return
    }
    busy = true
    try {
      await api.signIn(serverUrl.trim(), showAdvanced && authOverride.trim() ? authOverride.trim() : undefined)
      await setSignedIn(serverUrl.trim())
    } catch (e) { error = String(e) } finally { busy = false }
  }
</script>

<div class="signin">
  <div class="logo">◆</div>
  <h1>Lore Desktop</h1>
  <p class="muted sub">Sign in to your Lore server</p>

  <label>Server URL
    <input bind:value={serverUrl} placeholder="lore://host:41337" disabled={busy} />
  </label>

  <button class="ghost adv" onclick={() => (showAdvanced = !showAdvanced)}>
    {showAdvanced ? '▾ Advanced' : '▸ Advanced'}
  </button>
  {#if showAdvanced}
    <label>Auth service URL (optional)
      <input bind:value={authOverride} placeholder="https://host:8081" disabled={busy} />
    </label>
  {/if}

  <button class="primary big" onclick={go} disabled={busy}>
    {busy ? 'Complete sign-in in your browser…' : 'Sign in'}
  </button>
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .signin { max-width: 360px; margin: 12vh auto; padding: 0 20px; text-align: center; }
  .logo { font-size: 44px; color: var(--accent); line-height: 1; }
  h1 { margin: 8px 0 2px; }
  .sub { margin-top: 0; }
  label { display: block; margin: 12px 0; text-align: left; font-size: 12px; color: var(--muted); }
  label input { margin-top: 4px; }
  .adv { display: block; margin: 0 auto 4px; }
  .big { width: 100%; padding: 11px; margin-top: 10px; }
</style>
