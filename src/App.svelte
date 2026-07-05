<script lang="ts">
  import { onMount } from 'svelte'
  import { session, bootstrap } from './lib/session.svelte'
  import { repo, refreshStatus, refreshLocks } from './lib/repo.svelte'
  import { ui, setView } from './lib/ui.svelte'
  import SignIn from './lib/SignIn.svelte'
  import TitleBar from './lib/TitleBar.svelte'
  import RepoPicker from './lib/RepoPicker.svelte'
  import NavRail from './lib/NavRail.svelte'
  import Changes from './lib/Changes.svelte'
  import FilePreview from './lib/FilePreview.svelte'
  import History from './lib/History.svelte'
  import Merge from './lib/Merge.svelte'
  import Locks from './lib/Locks.svelte'
  import StatusBar from './lib/StatusBar.svelte'

  let selectedPath = $state<string | null>(null)

  onMount(bootstrap)

  // Reload status + locks whenever the selected repository changes.
  $effect(() => {
    session.config.currentRepo
    refreshStatus()
    refreshLocks()
  })

  const files = $derived(repo.status?.files ?? [])
  const selected = $derived(files.find((f) => f.path === selectedPath) ?? null)

  // Keep a valid selection; default to the first binary file (best shows the compare).
  $effect(() => {
    if (files.length && (selectedPath === null || !files.some((f) => f.path === selectedPath))) {
      selectedPath = (files.find((f) => f.isBinary) ?? files[0]).path
    }
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
        <NavRail />
        <div class="content">
          {#if ui.view === 'changes'}
            <div class="workarea">
              <Changes selectedPath={selectedPath} onselect={(p) => (selectedPath = p)} />
              <FilePreview file={selected} />
            </div>
          {:else if ui.view === 'history'}
            <History />
          {:else if ui.view === 'merge'}
            <Merge onclose={() => setView('history')} />
          {:else if ui.view === 'locks'}
            <Locks />
          {/if}
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
  .content { flex: 1; display: flex; overflow: hidden; min-width: 0; }
  .workarea { flex: 1; display: flex; overflow: hidden; }
</style>
