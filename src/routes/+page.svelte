<script lang="ts">
  import BookEditor from '$lib/components/BookEditor.svelte';
  import AccountsScreen from '$lib/screens/AccountsScreen.svelte';
  import BookScreen from '$lib/screens/BookScreen.svelte';
  import BrowseScreen from '$lib/screens/BrowseScreen.svelte';
  import CatalogScreen from '$lib/screens/CatalogScreen.svelte';
  import DevicesScreen from '$lib/screens/DevicesScreen.svelte';
  import LibraryScreen from '$lib/screens/LibraryScreen.svelte';
  import SearchScreen from '$lib/screens/SearchScreen.svelte';
  import SourcesScreen from '$lib/screens/SourcesScreen.svelte';
  import WishlistScreen from '$lib/screens/WishlistScreen.svelte';
  import { nav } from '$lib/stores/nav.svelte';

  const view = $derived(nav.view);
  // A new key replays the settle-in animation; book pages share one key so Previous/Next in the editor does not.
  const viewKey = $derived(
    nav.searching
      ? 'search'
      : view.name === 'library'
        ? `library:${view.scope?.kind ?? ''}:${view.scope?.key ?? ''}`
        : view.name === 'browse'
          ? `browse:${view.kind}`
          : view.name === 'catalog'
            ? `catalog:${view.book.key}:${view.book.title}`
            : view.name
  );
</script>

<svelte:head>
  <title>Shelf</title>
</svelte:head>

{#key viewKey}
<!-- If a screen fails to draw, show a way out instead of leaving the window stuck. -->
<svelte:boundary onerror={(error) => console.error('Screen failed to draw', error)}>
<div class="view-in">
{#if nav.searching}
  <SearchScreen />
{:else if view.name === 'library'}
  <LibraryScreen />
{:else if view.name === 'book'}
  <BookScreen id={view.id} back={view.back} />
  {#if nav.editing}
    <BookEditor id={view.id} />
  {/if}
{:else if view.name === 'catalog'}
  {#key view.book.key + view.book.title}
    <CatalogScreen book={view.book} back={view.back} />
  {/key}
{:else if view.name === 'browse'}
  {#key view.kind}
    <BrowseScreen kind={view.kind} />
  {/key}
{:else if view.name === 'wishlist'}
  <WishlistScreen />
{:else if view.name === 'devices'}
  <DevicesScreen />
{:else if view.name === 'sources'}
  <SourcesScreen />
{:else if view.name === 'accounts'}
  <AccountsScreen />
{/if}
</div>
{#snippet failed(error, reset)}
  <div class="screen-error" role="alert">
    <h2>Something went wrong showing this page</h2>
    <p class="muted">{error instanceof Error ? error.message : String(error)}</p>
    <div class="screen-error-actions">
      <button class="btn primary" onclick={() => { nav.go('library'); reset(); }}>Back to library</button>
      <button class="btn" onclick={reset}>Try again</button>
    </div>
  </div>
{/snippet}
</svelte:boundary>
{/key}

<style>
  .screen-error {
    max-width: 520px;
    margin: 64px auto;
    padding: 28px;
    border-radius: 24px;
    background: var(--glass-thick);
    box-shadow: var(--glass-shadow);
    text-align: center;
  }
  .screen-error h2 {
    margin-bottom: 8px;
  }
  .screen-error-actions {
    display: flex;
    gap: 10px;
    justify-content: center;
    margin-top: 18px;
  }
</style>
