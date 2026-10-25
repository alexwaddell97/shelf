<script lang="ts">
  import { untrack } from 'svelte';
  import { openUrl, sourceUrl, type Source } from '$lib/api';
  import BookCard from '$lib/components/BookCard.svelte';
  import CatalogCard from '$lib/components/CatalogCard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { looseKey } from '$lib/format';
  import { library } from '$lib/stores/library.svelte';
  import { nav } from '$lib/stores/nav.svelte';
  import { search } from '$lib/stores/search.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';

  const query = $derived(nav.query.trim());

  // Follow the header search box; the store debounces and cancels stale lookups.
  $effect(() => {
    const q = nav.query;
    untrack(() => search.input(q));
  });

  const libMatches = $derived.by(() => {
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    if (words.length === 0) return [];
    return library.books.filter((b) => {
      const hay = `${b.title} ${b.author} ${b.series ?? ''}`.toLowerCase();
      return words.every((w) => hay.includes(w));
    });
  });

  const owned = $derived(new Set(library.books.map((b) => looseKey(b.title, b.author))));
  const online = $derived(search.results.filter((c) => !owned.has(looseKey(c.title, c.author))));
  const sources = $derived(settings.sources.filter((s) => s.enabled));

  const KIND_LABEL: Record<Source['kind'], string> = { store: 'Store', free: 'Free', custom: 'Your source' };

  async function openSource(src: Source) {
    const url = sourceUrl(src, query);
    const ok = await attempt(`Could not open ${src.name}`, async () => {
      await openUrl(url);
      return true;
    });
    if (ok) toasts.info(`Opened ${src.name} in your browser`, url);
  }
</script>

<section aria-labelledby="search-h">
  <h1 id="search-h" class="page-title">Results for “{query}”</h1>

  <div class="group">
    <h2 class="eyebrow">In your library · {libMatches.length}</h2>
    {#if libMatches.length === 0}
      <p class="muted">Nothing in your library matches yet.</p>
    {:else}
      <div class="book-grid">
        {#each libMatches as book (book.id)}
          <BookCard {book} detailed={false} onopen={() => nav.openBook(book.id, { kind: 'library' }, libMatches.map((b) => b.id))} />
        {/each}
      </div>
    {/if}
  </div>

  <div class="group">
    <div>
      <h2 class="eyebrow">
        Not in your library{search.status === 'done' ? ` · ${online.length}` : ''}
      </h2>
      <p class="help">From Open Library. Click a book to see where you can buy it, or get it free.</p>
    </div>

    {#if search.status === 'loading' || search.status === 'idle'}
      <div class="loading" role="status">
        <span class="spin ring"></span>
        Searching the catalogue…
      </div>
    {:else if search.status === 'error'}
      <div class="error" role="alert">
        <div>
          <strong>The catalogue search didn't work.</strong>
          <span class="mono">{search.error}</span>
        </div>
        <button class="btn sm" onclick={() => search.retry()}>Try again</button>
      </div>
    {:else if online.length === 0}
      <p class="muted">No catalogue matches for this search.</p>
    {:else}
      <div class="book-grid">
        {#each online as book (book.key + book.title)}
          <CatalogCard {book} onopen={() => nav.openCatalog(book)} />
        {/each}
      </div>
    {/if}
  </div>

  <div class="group">
    <div>
      <h2 class="eyebrow">Search other sites</h2>
      <p class="help">
        Not finding it above? Search a site directly. Anything you download and put in a watched folder gets imported by Shelf.
      </p>
    </div>
    {#if sources.length === 0}
      <p class="muted">
        No sources are switched on.
        <button class="link-btn" onclick={() => nav.go('sources')}>Turn some on in Sources</button>.
      </p>
    {:else}
      <div class="sources">
        {#each sources as src (src.id)}
          <button class="source btn" onclick={() => openSource(src)}>
            <span class="letter">{src.name.charAt(0).toUpperCase()}</span>
            <span class="source-text">
              <span class="source-name">{src.name}</span>
              <span class="source-kind">{KIND_LABEL[src.kind]}</span>
            </span>
            <span class="ext"><Icon name="external" size={15} /></span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 36px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .help {
    margin-top: 6px;
    max-width: 70ch;
    color: var(--muted);
  }
  .loading {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-soft);
  }
  .ring {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--border-dashed);
    border-top-color: var(--accent);
  }
  .error {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 16px;
    border-radius: 12px;
    border: 1px solid var(--accent-border);
    background: var(--accent-bg);
    color: var(--accent-text);
  }
  .error div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .error .mono {
    font-size: 12px;
    word-break: break-word;
    color: var(--muted);
  }
  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .source {
    height: 48px;
    gap: 10px;
    padding: 0 14px 0 8px;
    background: var(--surface);
    border-color: var(--border);
  }
  .letter {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 7px;
    background: var(--surface-tile);
    font-weight: 700;
  }
  .source-text {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    line-height: 1.2;
  }
  .source-name {
    font-weight: 600;
  }
  .source-kind {
    font-size: 11px;
    color: var(--faint);
    font-weight: 400;
  }
  .ext {
    display: flex;
    margin-left: 4px;
    color: var(--faint);
  }
</style>
