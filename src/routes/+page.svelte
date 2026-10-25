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
</script>

<svelte:head>
  <title>Shelf</title>
</svelte:head>

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
