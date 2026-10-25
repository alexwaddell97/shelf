<script lang="ts">
  import BookCard from '$lib/components/BookCard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { plural } from '$lib/format';
  import { imports } from '$lib/stores/imports.svelte';
  import { library, SORT_OPTIONS, type LibraryFilter, type LibrarySort } from '$lib/stores/library.svelte';
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
    <p class="page-sub">Loading your books…</p>
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
      <p class="hint">Works with .epub, .kepub, .mobi, .azw3 and .pdf. Set up a watch folder under Sources to import new downloads automatically.</p>
    </div>
  {:else}
    <div class="top">
      <div>
        <h1 id="lib-h" class="page-title">{scope ? scope.title : 'Library'}</h1>
        <p class="page-sub">
          {plural(visible.length, 'book')}{scope ? '' : ` · ${library.onDeviceCount} on a device`}
        </p>
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
      <div class="book-grid">
        {#each visible as book (book.id)}
          <BookCard
            {book}
            onopen={() => nav.openBook(book.id, scope ? { kind: 'scope', scope } : { kind: 'library' }, siblings)}
          />
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
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
    border-radius: 16px;
  }
  .welcome-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 64px;
    height: 64px;
    border-radius: 16px;
    background: var(--surface-tile);
    color: var(--accent);
  }
  .welcome h2 {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.01em;
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
