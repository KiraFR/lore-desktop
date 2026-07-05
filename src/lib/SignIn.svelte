<script lang="ts">
  import { api } from './api'
  import { DEFAULT_SERVER_URL, setSignedIn } from './session.svelte'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'

  let serverUrl = $state(DEFAULT_SERVER_URL)
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
    } catch (e) { toastError('Sign-in failed', e) } finally { busy = false }
  }
</script>

<div class="signin">
  <div class="mark"><Icon name="book" size={26} /></div>
  <h1>Welcome to Lore Desktop</h1>
  <p class="muted sub">Connect to your Lore server to browse, commit, and push.</p>

  <div class="form">
    <label for="server-url">Lore server</label>
    <input id="server-url" bind:value={serverUrl} placeholder="lore://host:41337" disabled={busy} />

    <button class="disclose" onclick={() => (showAdvanced = !showAdvanced)}>
      <Icon name={showAdvanced ? 'chevronDown' : 'chevronRight'} size={14} /> Advanced
    </button>
    {#if showAdvanced}
      <input bind:value={authOverride} placeholder="Auth service URL (optional)" disabled={busy} />
    {/if}

    <button class="accent go" onclick={go} disabled={busy}>
      <Icon name="external" size={16} />
      {busy ? 'Complete sign-in in your browser…' : 'Sign in'}
    </button>
    {#if error}<p class="error">{error}</p>{/if}
    <p class="dim hint">Signs in through your server's SSO · opens your browser</p>
  </div>
</div>

<style>
  .signin { max-width: 340px; margin: 12vh auto; padding: 0 20px; text-align: center; }
  .mark { width: 54px; height: 54px; border-radius: 50%; background: var(--accent-soft); color: var(--accent); display: grid; place-items: center; margin: 0 auto 16px; }
  h1 { font-size: 20px; font-weight: 500; margin: 0 0 6px; }
  .sub { margin: 0 0 26px; font-size: 13px; }
  .form { text-align: left; }
  label { display: block; font-size: 12px; color: var(--text-muted); margin-bottom: 6px; }
  .disclose { display: inline-flex; align-items: center; gap: 4px; background: none; border: none; color: var(--accent); padding: 9px 0 0; }
  .disclose:hover { background: none; }
  .form input + .disclose { margin-top: 0; }
  .form input:nth-of-type(2) { margin-top: 8px; }
  .go { width: 100%; margin-top: 16px; padding: 10px; display: flex; align-items: center; justify-content: center; gap: 7px; font-weight: 500; }
  .hint { text-align: center; font-size: 12px; margin: 12px 0 0; }
  .error { text-align: center; margin: 10px 0 0; }
</style>
