<script lang="ts">
  import '@fontsource-variable/geist';
  import '@fontsource-variable/geist-mono';
  import '@fontsource-variable/literata';
  import '../app.css';
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import CalibreConfirm from '$lib/components/CalibreConfirm.svelte';
  import BookMenu from '$lib/components/BookMenu.svelte';
  import CalibrePanel from '$lib/components/CalibrePanel.svelte';
  import DropOverlay from '$lib/components/DropOverlay.svelte';
  import ImportPanel from '$lib/components/ImportPanel.svelte';
  import SetupWizard from '$lib/components/SetupWizard.svelte';
  import Spotlight from '$lib/components/Spotlight.svelte';
  import SyncPanel from '$lib/components/SyncPanel.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toasts from '$lib/components/Toasts.svelte';
  import TopBar from '$lib/components/TopBar.svelte';
  import { startApp } from '$lib/stores/app.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { theme } from '$lib/stores/theme.svelte';

  let { children }: { children: Snippet } = $props();

  // Reader windows show just the book: no sidebar, top bar or library background work.
  const reading = page.route.id?.startsWith('/reader') ?? false;

  onMount(() => {
    const stop = reading ? () => {} : startApp();
    const stopTheme = reading ? () => {} : theme.init();
    // This is a desktop app, so the browser's own right-click menu (Back, Reload, Inspect…)
    // makes no sense. Keep it only where it's useful: in text fields and over selected text.
    const quiet = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      const editable = target?.closest('input, textarea, [contenteditable="true"]');
      const selected = window.getSelection()?.toString();
      if (!editable && !selected) e.preventDefault();
    };
    document.addEventListener('contextmenu', quiet);
    return () => {
      document.removeEventListener('contextmenu', quiet);
      stop();
      stopTheme();
    };
  });
</script>

{#if reading}
  {@render children()}
{:else}
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
  <SyncPanel />
  <CalibrePanel />
  <ImportPanel />
</div>
<CalibreConfirm />
<BookMenu />
<Spotlight />
<DropOverlay />
<Toasts />
{/if}

<style>
  .app {
    display: grid;
    /* The sidebar floats inside its column, inset by the float gap. */
    grid-template-columns: calc(var(--nav-w) + var(--float-gap)) minmax(0, 1fr);
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
