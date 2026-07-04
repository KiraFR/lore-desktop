<script lang="ts">
  import { onMount } from 'svelte'
  import { session, bootstrap } from './lib/session.svelte'
  import { refreshStatus } from './lib/repo.svelte'
  import SignIn from './lib/SignIn.svelte'
  import TitleBar from './lib/TitleBar.svelte'
  import RepoPicker from './lib/RepoPicker.svelte'
  import Changes from './lib/Changes.svelte'
  import StatusBar from './lib/StatusBar.svelte'

  onMount(bootstrap)

  // Reload status whenever the selected repository changes.
  $effect(() => {
    session.config.currentRepo
    refreshStatus()
  })
</script>

<main class="shell">
  {#if !session.ready}
    <div class="fill muted">Loading…</div>
  {:else if !session.signedIn}
    <SignIn />
  {:else}
    <TitleBar />
    <div class="body">
      {#if session.config.currentRepo}
        <div class="workarea">
          <Changes />
          <div class="preview">
            <div class="ph muted">
              <p>Select a file to preview.</p>
              <p class="dim small">Binary before/after compare arrives in a later update.</p>
            </div>
          </div>
        </div>
      {:else}
        <RepoPicker />
      {/if}
    </div>
    <StatusBar />
  {/if}
</main>

<style>
  .shell { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }
  .fill { display: grid; place-items: center; flex: 1; }
  .body { flex: 1; overflow: hidden; display: flex; }
  .workarea { flex: 1; display: flex; overflow: hidden; }
  .preview { flex: 1; display: grid; place-items: center; padding: 20px; }
  .ph { text-align: center; }
  .ph .small { font-size: 12px; margin-top: 4px; }
</style>
