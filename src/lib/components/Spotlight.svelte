<script lang="ts">
  import { tick } from 'svelte';
  import type { Book } from '$lib/api';
  import { authorEntries, seriesLabel, statusLabel } from '$lib/format';
  import { fold, highlight, searchBooks, searchNames, tokenize } from '$lib/search';
  import { bookActions } from '$lib/stores/bookActions.svelte';
  import { devices } from '$lib/stores/devices.svelte';
  import { imports } from '$lib/stores/imports.svelte';
  import { labels } from '$lib/stores/labels.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { authorScope, collectionScope, nav, seriesScope } from '$lib/stores/nav.svelte';
  import { spotlight } from '$lib/stores/spotlight.svelte';
  import { theme } from '$lib/stores/theme.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';

  // A centred search window over the whole app: books, authors, series, collections and
  // commands. Enter opens; Ctrl+Enter on a book opens it straight in the reader.

  type IconName = 'library' | 'bookmark' | 'device' | 'user' | 'layers' | 'search' | 'plus' | 'sync' | 'grid' | 'list' | 'tag' | 'link' | 'folder';
  type Option =
    | { kind: 'book'; id: string; book: Book }
    | { kind: 'name'; id: string; icon: IconName; title: string; sub: string; run: () => void }
    | { kind: 'command'; id: string; icon: IconName; title: string; sub?: string; keywords: string; run: () => void };
  interface Group {
    title: string;
    options: Option[];
  }

  let text = $state('');
  let active = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let list = $state<HTMLDivElement | null>(null);

  const query = $derived(text.trim());
  const words = $derived(tokenize(text));

  function commands(): Option[] {
    const device = devices.connected[0];
    const go = (title: string, icon: IconName, section: Parameters<typeof nav.go>[0], keywords: string): Option => ({
      kind: 'command', id: `go-${section}`, icon, title, keywords, run: () => nav.go(section)
    });
    const all: Option[] = [];
    if (device) {
      all.push({
        kind: 'command', id: 'sync', icon: 'sync', title: `Sync with ${device.name}`,
        sub: devices.queueCount ? `${devices.queueCount} waiting to send` : 'Update reading progress',
        keywords: 'sync kobo device send ereader e-reader', run: () => devices.sync(device.id)
      });
    }
    all.push(
      { kind: 'command', id: 'add', icon: 'plus', title: 'Add books…', keywords: 'add import books files epub', run: () => imports.pickAndImport() },
      go('Go to Library', 'library', 'library', 'library books shelf home'),
      go('Go to Wishlist', 'bookmark', 'wishlist', 'wishlist wanted to read'),
      go('Go to Devices', 'device', 'devices', 'devices kobo ereader e-reader'),
      go('Go to Settings', 'link', 'sources', 'settings sources preferences appearance calibre'),
      go('Go to Accounts', 'user', 'accounts', 'accounts goodreads hardcover'),
      { kind: 'command', id: 'view-shelf', icon: 'library', title: 'Show as bookshelf', keywords: 'view shelf bookshelf layout', run: () => (library.setView('shelf'), nav.go('library')) },
      { kind: 'command', id: 'view-grid', icon: 'grid', title: 'Show as grid', keywords: 'view grid layout covers', run: () => (library.setView('grid'), nav.go('library')) },
      { kind: 'command', id: 'view-list', icon: 'list', title: 'Show as list', keywords: 'view list layout table', run: () => (library.setView('list'), nav.go('library')) },
      {
        kind: 'command', id: 'theme', icon: 'layers',
        title: theme.resolved === 'dark' ? 'Switch to light theme' : 'Switch to dark theme',
        keywords: 'theme dark light mode appearance night', run: () => theme.set(theme.resolved === 'dark' ? 'light' : 'dark')
      }
    );
    return all;
  }

  const groups = $derived.by<Group[]>(() => {
    const out: Group[] = [];
    const cmds = commands();
    if (!query) {
      const reading = library.books
        .filter((b) => b.status === 'reading')
        .sort((a, b) => (b.lastRead ?? '').localeCompare(a.lastRead ?? ''))
        .slice(0, 4);
      const recent = [...library.books]
        .filter((b) => !reading.includes(b))
        .sort((a, b) => b.addedAt.localeCompare(a.addedAt))
        .slice(0, 4);
      if (reading.length) out.push({ title: 'Continue reading', options: reading.map((book) => ({ kind: 'book', id: `b${book.id}`, book })) });
      if (recent.length) out.push({ title: 'Recently added', options: recent.map((book) => ({ kind: 'book', id: `r${book.id}`, book })) });
      out.push({ title: 'Commands', options: cmds.slice(0, devices.connected.length ? 3 : 2) });
      return out;
    }

    const books = searchBooks(library.books, query, 7);
    if (books.length) out.push({ title: 'Books', options: books.map((book) => ({ kind: 'book', id: `b${book.id}`, book })) });

    const authorCounts = new Map<string, { key: string; count: number }>();
    for (const b of library.books)
      for (const a of authorEntries(b)) {
        const seen = authorCounts.get(a.name);
        authorCounts.set(a.name, { key: a.key, count: (seen?.count ?? 0) + 1 });
      }
    const seriesCounts = new Map<string, number>();
    for (const b of library.books) if (b.series) seriesCounts.set(b.series, (seriesCounts.get(b.series) ?? 0) + 1);
    const names: Option[] = [
      ...searchNames([...authorCounts.keys()], query, 3).map((name): Option => {
        const { key, count } = authorCounts.get(name)!;
        return { kind: 'name', id: `a${name}`, icon: 'user', title: name, sub: `Author · ${count} ${count === 1 ? 'book' : 'books'}`, run: () => nav.showScope(authorScope(key, name)) };
      }),
      ...searchNames([...seriesCounts.keys()], query, 3).map((name): Option => {
        const count = seriesCounts.get(name)!;
        return { kind: 'name', id: `s${name}`, icon: 'layers', title: name, sub: `Series · ${count} ${count === 1 ? 'book' : 'books'}`, run: () => nav.showScope(seriesScope(name)) };
      }),
      ...searchNames(labels.collections.map((c) => c.name), query, 3).map((name): Option => {
        const count = labels.collections.find((c) => c.name === name)?.count ?? 0;
        return { kind: 'name', id: `c${name}`, icon: 'folder', title: name, sub: `Collection · ${count} ${count === 1 ? 'book' : 'books'}`, run: () => nav.showScope(collectionScope(name)) };
      })
    ];
    if (names.length) out.push({ title: 'Authors, series and collections', options: names });

    const matching = cmds.filter((c) => c.kind === 'command' && words.every((w) => fold(`${c.title} ${c.keywords}`).includes(w)));
    out.push({
      title: 'Commands',
      options: [
        ...matching,
        {
          kind: 'command', id: 'everything', icon: 'search', title: `Search everything for “${query}”`,
          sub: 'Your library, Open Library, Project Gutenberg and your stores', keywords: '', run: () => (nav.query = query)
        }
      ]
    });
    return out;
  });

  const options = $derived(groups.flatMap((g) => g.options));

  $effect(() => {
    void options;
    active = 0;
  });

  // Fresh each time it opens.
  $effect(() => {
    if (!spotlight.open) return;
    text = '';
    void tick().then(() => input?.focus());
  });

  function close() {
    spotlight.hide();
  }

  function choose(option: Option, read = false) {
    close();
    if (option.kind === 'book') {
      if (read) void bookActions.read(option.book);
      else nav.openBook(option.book.id, { kind: 'library' }, options.filter((o) => o.kind === 'book').map((o) => (o as { book: Book }).book.id));
    } else {
      void option.run();
    }
  }

  async function scrollToActive() {
    await tick();
    list?.querySelector<HTMLElement>(`#spot-${active}`)?.scrollIntoView({ block: 'nearest' });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = (active + 1) % options.length;
      void scrollToActive();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = (active - 1 + options.length) % options.length;
      void scrollToActive();
    } else if (e.key === 'Enter' && options[active]) {
      e.preventDefault();
      choose(options[active], e.ctrlKey || e.metaKey);
    } else if (e.key === 'Tab') {
      e.preventDefault();
    }
  }

  function onwindowkey(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && (e.key.toLowerCase() === 'k' || e.key === ' ')) {
      e.preventDefault();
      spotlight.toggle();
    }
  }
</script>

<svelte:window onkeydown={onwindowkey} />

{#snippet marked(value: string)}
  {#each highlight(value, words) as piece, i (i)}{#if piece.hit}<mark>{piece.text}</mark>{:else}{piece.text}{/if}{/each}
{/snippet}

{#if spotlight.open}
  <div class="scrim" role="presentation" onpointerdown={close}></div>
  <!-- Keep focus in the field (and so the keyboard working) when clicking non-controls. -->
  <div
    class="spotlight"
    role="dialog"
    aria-modal="true"
    aria-label="Spotlight search"
    tabindex="-1"
    onpointerdown={(e) => {
      if (!(e.target as HTMLElement).closest('input, button, a')) e.preventDefault();
    }}
  >
    <div class="spot-field">
      <Icon name="search" size={20} />
      <input
        bind:this={input}
        bind:value={text}
        role="combobox"
        aria-expanded="true"
        aria-controls="spot-list"
        aria-activedescendant={options.length ? `spot-${active}` : undefined}
        aria-label="Search books, authors, series and commands"
        placeholder="Search books, authors, series, or a command…"
        autocomplete="off"
        spellcheck="false"
        {onkeydown}
      />
      <kbd class="mono">esc</kbd>
    </div>

    <div class="results" id="spot-list" role="listbox" aria-label="Results" bind:this={list}>
      {#if query && !groups.some((g) => g.title === 'Books')}
        <p class="none">No books in your library match “{query}”.</p>
      {/if}
      {#each groups as group (group.title)}
        <div class="group">{group.title}</div>
        {#each group.options as option (option.id)}
          {@const i = options.indexOf(option)}
          <div
            id="spot-{i}"
            class="row"
            class:active={i === active}
            role="option"
            aria-selected={i === active}
            tabindex="-1"
            onpointerdown={(e) => e.preventDefault()}
            onpointermove={() => (active = i)}
            onclick={(e) => choose(option, e.ctrlKey || e.metaKey)}
            onkeydown={() => {}}
          >
            {#if option.kind === 'book'}
              {@const b = option.book}
              {@const s = seriesLabel(b)}
              <span class="thumb"><Cover title={b.title} author={b.author} coverPath={b.coverPath} /></span>
              <span class="lines">
                <span class="title serif">{@render marked(b.title)}</span>
                <span class="sub">
                  {@render marked(b.author)}{#if s}<span class="dot">·</span><span class="series">{@render marked(s)}</span>{/if}
                </span>
              </span>
              <span class="meta">
                {#if b.status === 'reading'}<span class="bar"><span style:width="{b.progress}%"></span></span>{/if}
                <span>{statusLabel(b)}</span>
              </span>
            {:else}
              <span class="glyph" class:accent={option.id === 'everything' || option.id === 'sync'}><Icon name={option.icon} size={17} /></span>
              <span class="lines">
                <span class="title">{#if option.kind === 'name'}{@render marked(option.title)}{:else}{option.title}{/if}</span>
                {#if option.sub}<span class="sub">{option.sub}</span>{/if}
              </span>
            {/if}
          </div>
        {/each}
      {/each}
    </div>

    <div class="keys mono" aria-hidden="true">
      <span>↑↓ move</span><span>↵ open</span><span>Ctrl ↵ read</span><span>esc close</span>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: color-mix(in srgb, var(--scrim) 55%, transparent);
    backdrop-filter: blur(6px);
    animation: fade 0.14s ease-out;
  }
  .spotlight {
    position: fixed;
    z-index: 71;
    top: 12vh;
    left: 50%;
    width: min(680px, calc(100vw - 32px));
    max-height: 72vh;
    display: flex;
    flex-direction: column;
    transform: translateX(-50%);
    border-radius: 26px;
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    border: 0;
    box-shadow: var(--glass-shadow-lift);
    overflow: hidden;
    animation: rise 0.18s ease-out;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, -8px) scale(0.985);
    }
  }
  /* Not ".field": that's the app's shared form-field class, which stacks its contents. */
  .spot-field {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 12px;
    padding: 0 18px;
    height: 62px;
    border-bottom: 1px solid var(--border-faint);
    color: var(--faint);
  }
  .spot-field input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 18px;
    text-align: left;
  }
  kbd {
    padding: 2px 7px;
    border-radius: 6px;
    border: 1px solid var(--border);
    font-size: 11px;
    color: var(--faint);
  }
  .results {
    flex: 1;
    overflow-y: auto;
    padding: 6px 8px 8px;
  }
  .group {
    padding: 12px 10px 5px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--faint);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 13px;
    min-height: 50px;
    padding: 6px 10px;
    border-radius: 11px;
    cursor: pointer;
  }
  .row.active {
    background: var(--accent-bg);
  }
  .thumb {
    flex: none;
    width: 32px;
    border-radius: 3px;
    overflow: hidden;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
  }
  .glyph {
    flex: none;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
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
    font-size: 15px;
    font-weight: 600;
  }
  .title.serif {
    font-family: var(--font-serif);
  }
  .sub {
    font-size: 12.5px;
    color: var(--muted);
  }
  .dot {
    margin: 0 5px;
  }
  .series {
    color: var(--accent);
  }
  .meta {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    font-size: 12px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .bar {
    display: block;
    width: 56px;
    height: 3px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  mark {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: inherit;
    border-radius: 3px;
    padding: 0 1px;
  }
  .none {
    margin: 0;
    padding: 14px 10px 2px;
    font-size: 13px;
    color: var(--muted);
  }
  .keys {
    display: flex;
    gap: 16px;
    justify-content: flex-end;
    padding: 9px 16px;
    border-top: 1px solid var(--border-faint);
    font-size: 11px;
    color: var(--faint);
  }
  @media (prefers-reduced-motion: reduce) {
    .scrim,
    .spotlight {
      animation: none;
    }
  }
</style>
