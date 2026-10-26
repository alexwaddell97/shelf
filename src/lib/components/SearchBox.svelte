<script lang="ts">
  import type { Book } from '$lib/api';
  import { authorEntries, seriesLabel } from '$lib/format';
  import { highlight, searchBooks, searchNames, tokenize } from '$lib/search';
  import { library } from '$lib/stores/library.svelte';
  import { authorScope, nav, seriesScope } from '$lib/stores/nav.svelte';
  import { spotlight } from '$lib/stores/spotlight.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';

  // The header search. Typing shows suggestions from the library underneath, leaving the
  // current screen alone; "Search everything" opens the full results with online books.
  // Once that full search is showing, typing updates it live instead.

  type Option =
    | { kind: 'book'; id: string; book: Book }
    | { kind: 'author'; id: string; name: string; key: string; count: number }
    | { kind: 'series'; id: string; name: string; count: number }
    | { kind: 'everything'; id: string };

  let text = $state(nav.query);
  let focused = $state(false);
  let active = $state(0);
  let input = $state<HTMLInputElement | null>(null);

  // Follow the full search when something else sets or clears it (Back, opening a book).
  $effect(() => {
    const q = nav.query;
    if (q && q !== text) text = q;
  });

  const words = $derived(tokenize(text));
  const query = $derived(text.trim());
  const open = $derived(focused && !!query && !nav.searching);

  const options = $derived.by<Option[]>(() => {
    if (!query) return [];
    const books = searchBooks(library.books, query, 6);

    const authorCounts = new Map<string, { key: string; count: number }>();
    for (const b of library.books) {
      for (const a of authorEntries(b)) {
        const seen = authorCounts.get(a.name);
        authorCounts.set(a.name, { key: a.key, count: (seen?.count ?? 0) + 1 });
      }
    }
    const seriesCounts = new Map<string, number>();
    for (const b of library.books) if (b.series) seriesCounts.set(b.series, (seriesCounts.get(b.series) ?? 0) + 1);

    return [
      ...books.map((book): Option => ({ kind: 'book', id: `b${book.id}`, book })),
      ...searchNames([...authorCounts.keys()], query, 3).map(
        (name): Option => ({ kind: 'author', id: `a${name}`, name, ...authorCounts.get(name)! })
      ),
      ...searchNames([...seriesCounts.keys()], query, 3).map(
        (name): Option => ({ kind: 'series', id: `s${name}`, name, count: seriesCounts.get(name)! })
      ),
      { kind: 'everything', id: 'everything' }
    ];
  });

  const books = $derived(options.filter((o) => o.kind === 'book'));
  const authors = $derived(options.filter((o) => o.kind === 'author'));
  const series = $derived(options.filter((o) => o.kind === 'series'));

  // Start at the top whenever the suggestions change.
  $effect(() => {
    void options;
    active = 0;
  });

  function finish() {
    text = '';
    input?.blur();
  }

  function choose(option: Option) {
    switch (option.kind) {
      case 'book': {
        const ids = books.map((o) => (o.kind === 'book' ? o.book.id : 0));
        finish();
        nav.openBook(option.book.id, { kind: 'library' }, ids);
        break;
      }
      case 'author':
        finish();
        nav.showScope(authorScope(option.key, option.name));
        break;
      case 'series':
        finish();
        nav.showScope(seriesScope(option.name));
        break;
      case 'everything':
        nav.query = query;
        input?.blur();
        break;
    }
  }

  function oninput() {
    // The full results are showing: keep them in step with the box.
    if (nav.searching) nav.query = text;
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (text || nav.query) {
        e.preventDefault();
        e.stopPropagation();
        text = '';
        nav.query = '';
      } else input?.blur();
      return;
    }
    if (!open || !options.length) {
      if (e.key === 'Enter' && query) nav.query = query;
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = (active + 1) % options.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = (active - 1 + options.length) % options.length;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      choose(options[active]);
    }
  }

  const indexOf = (option: Option) => options.indexOf(option);
</script>

{#snippet marked(value: string)}
  {#each highlight(value, words) as piece, i (i)}{#if piece.hit}<mark>{piece.text}</mark>{:else}{piece.text}{/if}{/each}
{/snippet}

{#snippet row(option: Option)}
  {@const i = indexOf(option)}
  <li
    id="suggest-{i}"
    role="option"
    aria-selected={i === active}
    class:active={i === active}
    onpointerdown={(e) => e.preventDefault()}
    onpointerenter={() => (active = i)}
    onclick={() => choose(option)}
    onkeydown={() => {}}
  >
    {#if option.kind === 'book'}
      {@const b = option.book}
      {@const s = seriesLabel(b)}
      <span class="thumb"><Cover title={b.title} author={b.author} coverPath={b.coverPath} /></span>
      <span class="lines">
        <span class="title">{@render marked(b.title)}</span>
        <span class="sub">{@render marked(b.author)}{#if s}<span class="dot">·</span><span class="series">{@render marked(s)}</span>{/if}</span>
      </span>
    {:else if option.kind === 'author'}
      <span class="glyph"><Icon name="user" size={16} /></span>
      <span class="lines">
        <span class="title">{@render marked(option.name)}</span>
        <span class="sub">{option.count} {option.count === 1 ? 'book' : 'books'}</span>
      </span>
    {:else if option.kind === 'series'}
      <span class="glyph"><Icon name="layers" size={16} /></span>
      <span class="lines">
        <span class="title">{@render marked(option.name)}</span>
        <span class="sub">Series · {option.count} {option.count === 1 ? 'book' : 'books'}</span>
      </span>
    {:else}
      <span class="glyph accent"><Icon name="search" size={16} /></span>
      <span class="lines">
        <span class="title">Search everything for “{query}”</span>
        <span class="sub">Your library, Open Library, Project Gutenberg and your stores</span>
      </span>
    {/if}
  </li>
{/snippet}

<div class="search" class:open>
  <span class="search-icon"><Icon name="search" /></span>
  <label for="shelf-search" class="sr-only">Search books</label>
  <input
    bind:this={input}
    bind:value={text}
    id="shelf-search"
    type="search"
    role="combobox"
    aria-expanded={open}
    aria-controls="search-suggestions"
    aria-autocomplete="list"
    aria-activedescendant={open ? `suggest-${active}` : undefined}
    placeholder="Search your library, or find new books"
    autocomplete="off"
    spellcheck="false"
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
    {oninput}
    {onkeydown}
  />
  <button class="hint mono" title="Open Spotlight search (Ctrl+K)" onclick={() => spotlight.show()}>Ctrl K</button>

  {#if open}
    <div class="panel" id="search-suggestions" role="listbox" aria-label="Suggestions">
      {#if books.length}
        <div class="group">In your library</div>
        <ul>{#each books as option (option.id)}{@render row(option)}{/each}</ul>
      {:else}
        <p class="none">Nothing in your library matches “{query}”.</p>
      {/if}
      {#if authors.length}
        <div class="group">Authors</div>
        <ul>{#each authors as option (option.id)}{@render row(option)}{/each}</ul>
      {/if}
      {#if series.length}
        <div class="group">Series</div>
        <ul>{#each series as option (option.id)}{@render row(option)}{/each}</ul>
      {/if}
      <ul class="everything">{@render row(options[options.length - 1])}</ul>
      <div class="keys mono" aria-hidden="true"><span>↑↓ move</span><span>↵ open</span><span>esc close</span></div>
    </div>
  {/if}
</div>

<style>
  .search {
    position: relative;
    flex: 1 1 320px;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 44px;
    padding: 0 16px;
    background: var(--fill);
    border: 0;
    border-radius: 999px;
    transition:
      background var(--dur) var(--ease),
      box-shadow var(--dur) var(--ease);
  }
  .search:focus-within {
    background: var(--surface);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .search-icon {
    display: flex;
    color: var(--faint);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--text);
    font-size: 15px;
  }
  input:focus-visible {
    outline: none;
  }
  .hint {
    flex: none;
    background: transparent;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 6px;
    border: 0;
    color: var(--faint);
    font-size: 11px;
  }
  .search:focus-within .hint {
    display: none;
  }

  .panel {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    right: 0;
    z-index: 30;
    max-height: min(560px, calc(100vh - 120px));
    overflow-y: auto;
    padding: 6px;
    border-radius: 20px;
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    border: 0;
    box-shadow: var(--glass-shadow);
    animation: drop 0.14s ease-out;
  }
  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  .group {
    padding: 10px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--faint);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 10px;
    border-radius: 12px;
    cursor: pointer;
  }
  li.active {
    background: var(--accent-bg);
  }
  .thumb {
    flex: none;
    width: 28px;
    border-radius: 3px;
    overflow: hidden;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
  }
  .glyph {
    flex: none;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: var(--hover-wash);
    color: var(--muted);
  }
  .glyph.accent {
    color: var(--accent);
  }
  .lines {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .title,
  .sub {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .title {
    font-weight: 600;
    font-size: 14px;
  }
  .sub {
    font-size: 12px;
    color: var(--muted);
  }
  .dot {
    margin: 0 5px;
  }
  .series {
    color: var(--accent);
  }
  mark {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: inherit;
    border-radius: 3px;
    padding: 0 1px;
  }
  .none {
    margin: 0;
    padding: 12px 10px 6px;
    font-size: 13px;
    color: var(--muted);
  }
  .everything {
    margin-top: 6px;
    padding-top: 6px;
    border-top: 1px solid var(--border-faint);
  }
  .keys {
    display: flex;
    gap: 14px;
    justify-content: flex-end;
    padding: 8px 10px 4px;
    font-size: 11px;
    color: var(--faint);
  }
  @media (prefers-reduced-motion: reduce) {
    .panel {
      animation: none;
    }
  }
</style>
