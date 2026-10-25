<script lang="ts">
  import '@fontsource-variable/geist';
  import '@fontsource-variable/geist-mono';
  import '../app.css';
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import CalibreConfirm from '$lib/components/CalibreConfirm.svelte';
  import CalibrePanel from '$lib/components/CalibrePanel.svelte';
  import DropOverlay from '$lib/components/DropOverlay.svelte';
  import ImportPanel from '$lib/components/ImportPanel.svelte';
  import SetupWizard from '$lib/components/SetupWizard.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toasts from '$lib/components/Toasts.svelte';
  import TopBar from '$lib/components/TopBar.svelte';
  import { startApp } from '$lib/stores/app.svelte';
  import { settings } from '$lib/stores/settings.svelte';

  let { children }: { children: Snippet } = $props();

  onMount(() => startApp());
</script>

<div class="app">
  <Sidebar />
  <main>
    <TopBar />
    <div class="content">
      {@render children()}
    </div>
  </main>
</div>

{#if settings.showSetup}
  <SetupWizard />
{/if}
<div class="floating">
  <CalibrePanel />
  <ImportPanel />
</div>
<CalibreConfirm />
<DropOverlay />
<Toasts />

<style>
  .app {
    display: grid;
    grid-template-columns: var(--nav-w) minmax(0, 1fr);
    min-height: 100vh;
  }
  main {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .floating {
    position: fixed;
    right: 24px;
    bottom: 24px;
    z-index: 25;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: calc(100% - 32px);
    pointer-events: none;
  }
  .floating > :global(*) {
    pointer-events: auto;
  }
  .content {
    padding: 28px var(--pad-x) 96px;
  }
</style>
