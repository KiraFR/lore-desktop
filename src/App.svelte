<script lang="ts">
  import { onMount } from 'svelte'
  import { session, bootstrap } from './lib/session.svelte'
  import SignIn from './lib/SignIn.svelte'
  import TitleBar from './lib/TitleBar.svelte'
  import RepoPicker from './lib/RepoPicker.svelte'
  import Changes from './lib/Changes.svelte'

  onMount(bootstrap)
</script>

<main class="shell">
  {#if !session.ready}
    <div class="fill muted">Loading…</div>
  {:else if !session.signedIn}
    <SignIn />
  {:else}
    <TitleBar />
    {#if session.config.currentRepo}
      <Changes repoPath={session.config.currentRepo} />
    {:else}
      <RepoPicker />
    {/if}
  {/if}
</main>

<style>
  .shell { display: flex; flex-direction: column; height: 100vh; overflow: hidden; }
  .fill { display: grid; place-items: center; flex: 1; }
</style>
