<script lang="ts">
  import BookCard from '$lib/components/BookCard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ListView from '$lib/components/ListView.svelte';
  import ShelfView from '$lib/components/ShelfView.svelte';
  import type { Book } from '$lib/api';
  import { plural } from '$lib/format';
  import { imports } from '$lib/stores/imports.svelte';
  import { library, SORT_OPTIONS, type LibraryFilter, type LibrarySort, type LibraryView } from '$lib/stores/library.svelte';
  import { browseKindOf, nav } from '$lib/stores/nav.svelte';

  const FILTERS: [LibraryFilter, string][] = [
    ['all', 'All'],
    ['reading', 'Reading'],
    ['unread', 'Not started'],
    ['finished', 'Finished'],
    ['on-device', 'On a device'],
    ['not-on-device', 'Not on a device']
  ];

  const BROWSE_LABEL = { authors: 'Authors', series: 'Series', collections: 'Collections', tags: 'Tags' } as const;

  const scope = $derived(nav.view.name === 'library' ? nav.view.scope : undefined);

  // Inside a series or an author's books the natural order is by series position.
  let override = $state<LibrarySort | null>(null);
  $effect(() => {
    scope;
    override = null;
  });
  const scopeDefault = $derived<LibrarySort | null>(scope && (scope.kind === 'series' || scope.kind === 'author') ? 'series' : null);
  const sort = $derived(override ?? scopeDefault ?? library.sort);

  const visible = $derived(library.list(scope, sort));
  const siblings = $derived(visible.map((b) => b.id));

  const VIEWS: [LibraryView, string, 'library' | 'grid' | 'list'][] = [
    ['shelf', 'Shelf', 'library'],
    ['grid', 'Grid', 'grid'],
    ['list', 'List', 'list']
  ];

  const open = (book: Book) => nav.openBook(book.id, scope ? { kind: 'scope', scope } : { kind: 'library' }, siblings);

  function pickSort(value: LibrarySort) {
    if (scope) override = value;
    else library.setSort(value);
  }
</script>

<section aria-labelledby="lib-h">
  {#if scope}
    <button class="btn back" onclick={() => nav.browse(browseKindOf(scope))}>
      <Icon name="back" />
      {BROWSE_LABEL[browseKindOf(scope)]}
    </button>
  {/if}

  {#if !library.loaded}
    <h1 id="lib-h" class="page-title">Library</h1>
    <p class="sr-only" role="status">Loading your books…</p>
    <div class="book-grid" aria-hidden="true">
      {#each Array.from({ length: 12 }, (_, i) => i) as i (i)}
        <div class="sk-card">
          <div class="skeleton sk-cover"></div>
          <div class="skeleton sk-line"></div>
          <div class="skeleton sk-line short"></div>
        </div>
      {/each}
    </div>
  {:else if library.books.length === 0 && !scope}
    <h1 id="lib-h" class="page-title">Library</h1>
    <div class="welcome">
      <span class="welcome-icon"><Icon name="drop" size={32} /></span>
      <h2>Your library is empty</h2>
      <p>Drop ebooks anywhere on this window, or click Add books. Shelf reads the title, author and cover for you.</p>
      <button class="btn primary" onclick={() => imports.pickAndImport()}>
        <Icon name="plus" />
        Add books
      </button>
      <p class="hint">Works with .epub, .kepub, .mobi, .azw3 and .pdf. Set up a watch folder under Settings to import new downloads automatically.</p>
    </div>
  {:else}
    <div class="top">
      <div>
        <h1 id="lib-h" class="page-title">{scope ? scope.title : 'Library'}</h1>
        <p class="page-sub">
          {plural(visible.length, 'book')}{scope ? '' : ` · ${library.onDeviceCount} on a device`}
        </p>
      </div>
      <div class="tools">
        <div class="views" role="group" aria-label="Layout">
          {#each VIEWS as [id, label, icon] (id)}
            {@const on = library.view === id}
            <button class="view" class:on aria-pressed={on} title="{label} view" onclick={() => library.setView(id)}>
              <Icon name={icon} size={16} />
              <span>{label}</span>
            </button>
          {/each}
        </div>
      <div class="sort">
        <label for="shelf-sort">Sort</label>
        <select id="shelf-sort" class="input" value={sort} onchange={(e) => pickSort(e.currentTarget.value as LibrarySort)}>
          {#each SORT_OPTIONS as [value, label] (value)}
            <option {value}>{label}</option>
          {/each}
        </select>
      </div>
      </div>
    </div>

    <div class="chips" role="group" aria-label="Filter books">
      {#each FILTERS as [id, label] (id)}
        {@const on = library.filter === id}
        <button class="chip" class:on aria-pressed={on} onclick={() => (library.filter = id)}>
          {label}
          <span class="mono">{library.count(id, scope)}</span>
        </button>
      {/each}
    </div>

    {#if visible.length === 0}
      <p class="empty">{scope && library.filter === 'all' ? 'No books here any more.' : 'No books match this filter.'}</p>
    {:else}
      {#if library.view === 'shelf'}
        <ShelfView books={visible} onopen={open} />
      {:else if library.view === 'list'}
        <ListView books={visible} onopen={open} />
      {:else}
        <div class="book-grid">
          {#each visible as book (book.id)}
            <BookCard {book} onopen={() => open(book)} />
          {/each}
        </div>
      {/if}
    {/if}
  {/if}
</section>

<style>
  .sk-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .sk-cover {
    aspect-ratio: 2 / 3;
    border-radius: var(--r1);
  }
  .sk-line {
    height: 12px;
    width: 85%;
  }
  .sk-line.short {
    width: 55%;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
  }
  .views {
    display: flex;
    padding: 3px;
    border-radius: var(--r2);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .view {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 34px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r2);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .view:hover {
    color: var(--text);
  }
  .view.on {
    background: var(--surface-tile);
    color: var(--text);
  }
  .sort {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .sort label {
    font-size: 13px;
    color: var(--muted);
  }
  .sort select {
    width: auto;
    background: var(--surface);
    border-color: var(--border);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 14px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-soft);
    font-weight: 500;
  }
  .chip .mono {
    font-size: 11px;
    color: var(--faint);
  }
  .chip.on {
    background: var(--text);
    border-color: var(--text);
    color: var(--bg);
  }
  .chip.on .mono {
    color: var(--bg);
    opacity: 0.6;
  }

  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    padding: 56px 24px;
    text-align: center;
    border: 1px dashed var(--border);
    border-radius: var(--r3);
  }
  .welcome-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 64px;
    height: 64px;
    border-radius: var(--r3);
    background: var(--surface-tile);
    color: var(--accent);
  }
  .welcome h2 {
    font-family: var(--font-serif);
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.005em;
  }
  .welcome p {
    max-width: 52ch;
    color: var(--muted);
  }
  .welcome .hint {
    font-size: 13px;
    color: var(--faint);
  }
</style>
