<script lang="ts" module>
  import type { LibrarySort, Scope } from '$lib/stores/library.svelte';

  type ColumnId = 'author' | 'series' | 'status' | 'device' | 'rating' | 'published' | 'added' | 'lastRead' | 'format' | 'size' | 'language';

  interface Column {
    id: ColumnId;
    label: string;
    sort: LibrarySort;
    /** CSS grid track. */
    track: string;
    /** Narrowest it can go, for deciding what fits. */
    min: number;
    /** Lower numbers stay longest when the window is narrow. */
    priority: number;
    end?: boolean;
  }

  // In display order. Title always comes first.
  const COLUMNS: Column[] = [
    { id: 'author', label: 'Author', sort: 'author', track: 'minmax(0, 1.3fr)', min: 110, priority: 1 },
    { id: 'series', label: 'Series', sort: 'series', track: 'minmax(0, 1.2fr)', min: 110, priority: 4 },
    { id: 'status', label: 'Progress', sort: 'status', track: 'minmax(96px, 118px)', min: 96, priority: 2 },
    { id: 'device', label: 'Where', sort: 'device', track: 'minmax(124px, 150px)', min: 124, priority: 3 },
    { id: 'rating', label: 'Rating', sort: 'rating', track: '84px', min: 84, priority: 6 },
    { id: 'published', label: 'Published', sort: 'published', track: '84px', min: 84, priority: 7 },
    { id: 'added', label: 'Added', sort: 'recent', track: '96px', min: 96, priority: 5 },
    { id: 'lastRead', label: 'Last read', sort: 'lastRead', track: '92px', min: 92, priority: 8 },
    { id: 'format', label: 'Format', sort: 'format', track: '64px', min: 64, priority: 9 },
    { id: 'size', label: 'Size', sort: 'size', track: '76px', min: 76, priority: 10, end: true },
    { id: 'language', label: 'Language', sort: 'language', track: '100px', min: 100, priority: 11 }
  ];
  const DEFAULT_COLUMNS: ColumnId[] = ['author', 'series', 'status', 'device', 'added'];

  const COLUMNS_KEY = 'shelf.listColumns';
  const COMPACT_KEY = 'shelf.listCompact';

  function loadColumns(): ColumnId[] {
    try {
      const saved = JSON.parse(localStorage.getItem(COLUMNS_KEY) ?? 'null');
      if (Array.isArray(saved)) return COLUMNS.map((c) => c.id).filter((id) => saved.includes(id));
    } catch {
      /* storage unavailable or not JSON */
    }
    return DEFAULT_COLUMNS;
  }

  function loadCompact(): boolean {
    try {
      return localStorage.getItem(COMPACT_KEY) === '1';
    } catch {
      return false;
    }
  }

  function save(key: string, value: string) {
    try {
      localStorage.setItem(key, value);
    } catch {
      /* storage unavailable */
    }
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import type { Book } from '$lib/api';
  import {
    authorEntries,
    authorSortKey,
    formatBytes,
    formatDate,
    languageName,
    relativeDate,
    seriesLabel,
    sortTitle,
    statusLabel
  } from '$lib/format';
  import { popover } from '$lib/popover';
  import { bookActions } from '$lib/stores/bookActions.svelte';
  import { devices } from '$lib/stores/devices.svelte';
  import { authorScope, nav, seriesScope } from '$lib/stores/nav.svelte';
  import ClampText from './ClampText.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import Stars from './Stars.svelte';
  import { tip } from '$lib/tooltip';

  // The library as a table, with the selected book's cover and details in a side panel.
  // Click selects, double-click or Enter opens, arrow keys move, typing jumps to a title.
  // Column headings sort (click again to reverse); the columns button picks which show.

  interface Props {
    books: Book[];
    onopen: (book: Book) => void;
    sort: LibrarySort;
    ascending: boolean;
    onsort: (sort: LibrarySort) => void;
  }

  let { books, onopen, sort, ascending, onsort }: Props = $props();

  let selectedId = $state<number | null>(null);
  const selected = $derived(books.find((b) => b.id === selectedId) ?? books[0] ?? null);
  let listEl = $state<HTMLDivElement | null>(null);

  let chosen = $state<ColumnId[]>(loadColumns());
  let compact = $state(loadCompact());
  let menuOpen = $state(false);
  let width = $state(0);

  // The chosen columns that fit, dropping the least important first when the list is narrow.
  const shown = $derived.by(() => {
    const picked = COLUMNS.filter((c) => chosen.includes(c.id));
    if (!width) return picked;
    const GAP = 14;
    // Row padding (the right side holds the columns button), the cover and the title.
    let room = width - 64 - (compact ? 0 : 40 + GAP) - 170;
    const keep = new Set<ColumnId>();
    for (const c of [...picked].sort((a, b) => a.priority - b.priority)) {
      if (c.min + GAP > room) continue;
      room -= c.min + GAP;
      keep.add(c.id);
    }
    return picked.filter((c) => keep.has(c.id));
  });
  const hidden = $derived(chosen.filter((id) => !shown.some((c) => c.id === id)));
  const has = (id: ColumnId) => shown.some((c) => c.id === id);
  const tracks = $derived([compact ? null : '40px', 'minmax(0, 2.2fr)', ...shown.map((c) => c.track)].filter(Boolean).join(' '));

  function toggleColumn(id: ColumnId) {
    chosen = chosen.includes(id) ? chosen.filter((c) => c !== id) : COLUMNS.map((c) => c.id).filter((c) => c === id || chosen.includes(c));
    save(COLUMNS_KEY, JSON.stringify(chosen));
  }

  function setCompact(on: boolean) {
    compact = on;
    save(COMPACT_KEY, on ? '1' : '0');
  }

  function resetColumns() {
    chosen = DEFAULT_COLUMNS;
    save(COLUMNS_KEY, JSON.stringify(chosen));
    setCompact(false);
  }

  async function select(book: Book, scroll = false) {
    selectedId = book.id;
    if (!scroll) return;
    await tick();
    listEl?.querySelector<HTMLElement>(`[data-id="${book.id}"]`)?.scrollIntoView({ block: 'nearest' });
  }

  // Type-to-find: letters typed in quick succession jump to the first match, on the
  // author or series when that's the sort, otherwise on the title.
  let typed = '';
  let typedTimer: ReturnType<typeof setTimeout> | undefined;
  const findKey = (b: Book) =>
    sort === 'author' ? authorSortKey(b) : sort === 'series' ? sortTitle(b.series ?? '') : sortTitle(b.title);

  function typeahead(key: string): boolean {
    if (key.length !== 1 || (key === ' ' && !typed)) return false;
    clearTimeout(typedTimer);
    typedTimer = setTimeout(() => (typed = ''), 900);
    typed += key.toLowerCase();
    const start = Math.max(0, books.findIndex((b) => b.id === selected?.id));
    // A single letter moves on to the next match; a longer string can stay put.
    const order = [...books.slice(start + (typed.length === 1 ? 1 : 0)), ...books.slice(0, start + 1)];
    const hit = order.find((b) => findKey(b).startsWith(typed));
    if (hit) void select(hit, true);
    return true;
  }

  function onkeydown(e: KeyboardEvent) {
    if (!selected || !books.length) return;
    const i = books.findIndex((b) => b.id === selected.id);
    const jump = (to: number) => {
      e.preventDefault();
      void select(books[Math.max(0, Math.min(books.length - 1, to))], true);
    };
    if (e.key === 'ArrowDown') jump(i + 1);
    else if (e.key === 'ArrowUp') jump(i - 1);
    else if (e.key === 'PageDown') jump(i + 10);
    else if (e.key === 'PageUp') jump(i - 10);
    else if (e.key === 'Home') jump(0);
    else if (e.key === 'End') jump(books.length - 1);
    else if (e.key === 'Enter') {
      e.preventDefault();
      onopen(selected);
    } else if (!e.ctrlKey && !e.metaKey && !e.altKey && typeahead(e.key)) e.preventDefault();
  }

  function oncontextmenu(e: MouseEvent, book: Book) {
    e.preventDefault();
    void select(book);
    bookActions.openMenu(e.clientX, e.clientY, book, () => onopen(book));
  }

  /** An author or series name in a row was clicked: show its books instead of selecting the row. */
  function go(e: MouseEvent, scope: Scope) {
    e.stopPropagation();
    nav.showScope(scope);
  }

  function headerMenu(e: MouseEvent) {
    e.preventDefault();
    menuOpen = true;
  }

  // The device action for the panel: the first connected device that applies.
  const deviceAction = $derived.by(() => {
    const book = selected;
    const device = devices.connected[0];
    if (!book || !device) return null;
    const on = book.onDevices.includes(device.id);
    const queued = book.queuedFor.includes(device.id);
    if (queued) return { label: on ? 'Cancel update' : 'Cancel sending', run: () => bookActions.cancelSend(book, device.id) };
    if (on) return { label: `Remove from ${device.name}`, run: () => bookActions.removeFromDevice(book, device) };
    return { label: `Send to ${device.name}`, run: () => bookActions.send(book, device), primary: true };
  });
</script>

{#snippet sorter(label: string, by: LibrarySort, end = false)}
  {@const active = sort === by}
  <button
    class="sorter"
    class:active
    class:end
    aria-label="Sort by {label.toLowerCase()}{active ? (ascending ? ', ascending' : ', descending') : ''}"
    onclick={() => onsort(by)}
  >
    {label}
    <span class="arrow" class:down={active && !ascending} aria-hidden="true"><Icon name="up" size={12} /></span>
  </button>
{/snippet}

<div class="layout">
  <div class="table" class:compact bind:clientWidth={width}>
    <div class="row head" style:grid-template-columns={tracks} role="group" aria-label="Sort by" oncontextmenu={headerMenu}>
      {#if !compact}<span></span>{/if}
      <span class="head-name">
        {@render sorter('Title', 'title')}
        {#if !has('author')}
          <span class="dot" aria-hidden="true">·</span>
          {@render sorter('Author', 'author')}
        {/if}
      </span>
      {#each shown as c (c.id)}
        <span class:end={c.end}>{@render sorter(c.label, c.sort, c.end)}</span>
      {/each}
      <span class="cols">
        <button class="cols-btn" class:on={menuOpen} aria-label="Choose columns" aria-expanded={menuOpen} use:tip={'Columns\nChoose what the list shows'} onclick={() => (menuOpen = !menuOpen)}>
          <Icon name="columns" size={15} />
        </button>
        {#if menuOpen}
          <div class="pop" role="menu" aria-label="Columns" tabindex="-1" use:popover={() => (menuOpen = false)}>
            <div class="pop-label">Columns</div>
            {#each COLUMNS as c (c.id)}
              {@const on = chosen.includes(c.id)}
              <button class="pop-item" role="menuitemcheckbox" aria-checked={on} onclick={() => toggleColumn(c.id)}>
                <span class="tick">{#if on}<Icon name="check" size={14} />{/if}</span>
                <span class="pop-text">{c.label}</span>
                {#if hidden.includes(c.id)}<span class="pop-note">No room</span>{/if}
              </button>
            {/each}
            <div class="pop-sep"></div>
            <button class="pop-item" role="menuitemcheckbox" aria-checked={compact} onclick={() => setCompact(!compact)}>
              <span class="tick">{#if compact}<Icon name="check" size={14} />{/if}</span>
              <span class="pop-text">Compact rows</span>
            </button>
            <button class="pop-item" role="menuitem" onclick={resetColumns}>
              <span class="tick"></span>
              <span class="pop-text">Reset to default</span>
            </button>
          </div>
        {/if}
      </span>
    </div>

    <div
      class="list"
      role="listbox"
      tabindex="0"
      aria-label="Books"
      aria-activedescendant={selected ? `book-row-${selected.id}` : undefined}
      bind:this={listEl}
      {onkeydown}
    >
      {#each books as book (book.id)}
        {@const sync = devices.bookSync(book)}
        {@const on = selected?.id === book.id}
        {@const series = seriesLabel(book)}
        <div
          id="book-row-{book.id}"
          data-id={book.id}
          class="row"
          class:on
          style:grid-template-columns={tracks}
          role="option"
          aria-selected={on}
          tabindex="-1"
          onclick={() => select(book)}
          ondblclick={() => onopen(book)}
          oncontextmenu={(e) => oncontextmenu(e, book)}
          onkeydown={() => {}}
        >
          {#if !compact}
            <span class="thumb"><Cover title={book.title} author={book.author} coverPath={book.coverPath} /></span>
          {/if}
          <!-- Author and series fall back to the title's second line when their columns don't fit. -->
          <span class="name">
            <span class="title" title={book.title}>{book.title}</span>
            {#if !has('author') || (!has('series') && series)}
              <span class="author">
                {#if !has('author')}{book.author}{/if}
                {#if !has('series') && series}{#if !has('author')}<span class="amp"> · </span>{/if}<span class="under-series mono">{series}</span>{/if}
              </span>
            {/if}
          </span>
          {#each shown as c (c.id)}
            {#if c.id === 'author'}
              <span class="cell" title={book.author}>
                {#each authorEntries(book) as a, i (a.key)}
                  {#if i > 0}<span class="amp"> &amp; </span>{/if}<span
                    class="link"
                    role="link"
                    tabindex="-1"
                    onclick={(e) => go(e, authorScope(a.key, a.name))}
                    ondblclick={(e) => e.stopPropagation()}
                    onkeydown={() => {}}>{a.name}</span
                  >
                {/each}
              </span>
            {:else if c.id === 'series'}
              <span class="cell" title={series}>
                {#if book.series}
                  <span
                    class="link series mono"
                    role="link"
                    tabindex="-1"
                    onclick={(e) => go(e, seriesScope(book.series!))}
                    ondblclick={(e) => e.stopPropagation()}
                    onkeydown={() => {}}>{series}</span
                  >
                {/if}
              </span>
            {:else if c.id === 'status'}
              <span class="status">
                {#if book.status === 'reading'}
                  <span class="mini-bar" aria-hidden="true"><span style:width="{book.progress}%"></span></span>
                {/if}
                <span class:done={book.status === 'finished'}>{statusLabel(book)}</span>
              </span>
            {:else if c.id === 'device'}
              <span class="cell where" class:device={sync.state === 'synced'} class:waiting={sync.state === 'waiting'} use:tip={sync.tip}>
                {#if sync.state !== 'pc'}<Icon name={sync.state === 'synced' ? 'check' : 'sync'} size={13} />{/if}
                {sync.label}
              </span>
            {:else if c.id === 'rating'}
              <span class="cell">{#if book.rating}<Stars value={book.rating} size={12} />{/if}</span>
            {:else if c.id === 'published'}
              <span class="cell mono small">{book.published?.slice(0, 4) ?? ''}</span>
            {:else if c.id === 'added'}
              <span class="cell mono small">{formatDate(book.addedAt)}</span>
            {:else if c.id === 'lastRead'}
              <span class="cell mono small">{book.lastRead ? relativeDate(book.lastRead) : ''}</span>
            {:else if c.id === 'format'}
              <span class="cell mono small">{book.format.toUpperCase()}</span>
            {:else if c.id === 'size'}
              <span class="cell mono small end">{formatBytes(book.sizeBytes)}</span>
            {:else if c.id === 'language'}
              <span class="cell small">{languageName(book.language) ?? ''}</span>
            {/if}
          {/each}
        </div>
      {/each}
    </div>
  </div>

  {#if selected}
    {@const book = selected}
    {@const series = seriesLabel(book)}
    {@const sync = devices.bookSync(book)}
    <aside class="panel" aria-label="Selected book">
      <button class="cover-btn" onclick={() => onopen(book)} aria-label="Open {book.title}">
        <Cover title={book.title} author={book.author} coverPath={book.coverPath} />
      </button>
      <div class="heading">
        {#if series}<div class="mono panel-series">{series}</div>{/if}
        <h2>{book.title}</h2>
        <p class="panel-author">{book.author}</p>
        {#if book.rating}<Stars value={book.rating} size={15} />{/if}
      </div>

      <div class="state">
        <div class="state-line">
          <span class:device={sync.state === 'synced'} use:tip={sync.tip}>{sync.label}</span>
          <span class="muted">{statusLabel(book)}</span>
        </div>
        {#if book.status === 'reading'}
          <div class="bar" aria-hidden="true"><span style:width="{book.progress}%"></span></div>
        {/if}
      </div>

      <div class="actions">
        <button class="btn primary" onclick={() => onopen(book)}>Open</button>
        <button class="btn" onclick={() => bookActions.edit(book, () => onopen(book))}>Edit details</button>
        {#if deviceAction}
          <button class="btn wide" class:teal={deviceAction.primary} onclick={deviceAction.run}>{deviceAction.label}</button>
        {/if}
        <button class="btn wide" onclick={() => bookActions.read(book)}>Read</button>
      </div>

      <dl>
        <div><dt>Format</dt><dd>{book.format.toUpperCase()} · {formatBytes(book.sizeBytes)}</dd></div>
        <div><dt>Added</dt><dd>{formatDate(book.addedAt)}</dd></div>
        {#if book.published}<div><dt>Published</dt><dd>{book.published}</dd></div>{/if}
        {#if book.publisher}<div><dt>Publisher</dt><dd>{book.publisher}</dd></div>{/if}
        {#if languageName(book.language)}<div><dt>Language</dt><dd>{languageName(book.language)}</dd></div>{/if}
        {#if book.isbn}<div><dt>ISBN</dt><dd class="mono">{book.isbn}</dd></div>{/if}
      </dl>

      {#if book.collections.length || book.tags.length}
        <div class="chips">
          {#each book.collections as name (name)}<span class="chip collection">{name}</span>{/each}
          {#each book.tags as name (name)}<span class="chip">{name}</span>{/each}
        </div>
      {/if}

      {#if book.description}
        <div class="description">
          {#key book.id}<ClampText text={book.description} lines={4} />{/key}
        </div>
      {/if}
    </aside>
  {/if}
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 24px;
    align-items: start;
  }
  @media (max-width: 1240px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
    .panel {
      display: none !important;
    }
  }

  /* No overflow clipping here: the heading row sticks to the page and the columns menu
     can hang past the end of a short list, so the corner rows round themselves. */
  .table {
    border-radius: 20px;
    background: var(--glass-tint);
    box-shadow: var(--glass-shine), 0 0 0 0.5px var(--border-soft);
  }
  .list {
    outline: none;
    border-radius: 0 0 20px 20px;
  }
  .list:focus-visible {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .list .row:last-child {
    border-radius: 0 0 20px 20px;
  }
  .row {
    display: grid;
    align-items: center;
    gap: 14px;
    min-height: 64px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border-faint);
    cursor: default;
    user-select: none;
  }
  .list .row:last-child {
    border-bottom: 0;
  }
  .list .row:hover {
    background: var(--surface-hover);
  }
  .row.on {
    background: var(--accent-bg-strong);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .compact .row {
    min-height: 40px;
    padding-top: 4px;
    padding-bottom: 4px;
  }

  /* Heading row: sticks under the top bar while the page scrolls. */
  .row.head {
    position: sticky;
    top: var(--topbar-h, 68px);
    z-index: 2;
    min-height: 40px;
    padding-top: 0;
    padding-bottom: 0;
    border-radius: 20px 20px 0 0;
    background: var(--surface-raised);
    border-bottom: 1px solid var(--border-soft);
  }
  .row.head > span {
    display: flex;
    min-width: 0;
  }
  .row.head > span.end {
    justify-content: flex-end;
  }
  .head-name {
    align-items: center;
    gap: 2px;
  }
  .dot {
    color: var(--faint);
  }
  .sorter {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    height: 28px;
    margin-left: -6px;
    padding: 0 6px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--faint);
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
    cursor: pointer;
  }
  .sorter.end {
    margin-left: 0;
    margin-right: -6px;
  }
  .sorter:hover {
    color: var(--text);
    background: var(--fill);
  }
  .sorter.active {
    color: var(--accent);
  }
  .arrow {
    display: inline-flex;
    opacity: 0;
    transition:
      transform var(--dur) var(--ease),
      opacity var(--dur) var(--ease);
  }
  .sorter:hover .arrow {
    opacity: 0.45;
  }
  .sorter.active .arrow {
    opacity: 1;
  }
  .arrow.down {
    transform: rotate(180deg);
  }

  /* The columns button sits past the last column, over the row's right padding. */
  .cols {
    position: absolute !important;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
  }
  .row,
  .compact .row {
    padding-right: 48px;
  }
  .cols-btn {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }
  .cols-btn:hover,
  .cols-btn.on {
    color: var(--text);
    background: var(--fill);
  }
  .pop {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 6;
    width: 220px;
    padding: 6px;
    border-radius: var(--radius-l);
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--glass-shadow-lift);
    text-transform: none;
    letter-spacing: 0;
  }
  .pop-label {
    padding: 6px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--faint);
  }
  .pop-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border: 0;
    border-radius: var(--radius-m);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
  }
  .pop-item:hover {
    background: var(--fill-hover);
  }
  .pop-text {
    flex: 1;
  }
  .pop-note {
    font-size: 11px;
    color: var(--faint);
  }
  .tick {
    display: inline-flex;
    width: 16px;
    color: var(--accent);
  }
  .pop-sep {
    height: 1px;
    margin: 6px 4px;
    background: var(--border-soft);
  }

  .thumb {
    display: block;
    width: 40px;
    box-shadow: var(--shadow-1);
    border-radius: 3px;
    overflow: hidden;
  }
  .name {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .compact .name {
    flex-direction: row;
    align-items: baseline;
    gap: 8px;
  }
  .title,
  .author,
  .cell {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .title {
    font-weight: 600;
  }
  .author {
    font-size: 13px;
    color: var(--muted);
  }
  .compact .author {
    flex: 0 1 auto;
  }
  .cell {
    min-width: 0;
    font-size: 13px;
    color: var(--text-soft);
  }
  .cell.small {
    font-size: 12px;
    color: var(--muted);
  }
  .cell.end {
    text-align: right;
  }
  .amp {
    color: var(--faint);
  }
  .under-series {
    font-size: 12px;
    color: var(--accent);
  }
  .link {
    cursor: pointer;
  }
  .link:hover {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .link.series {
    font-size: 12px;
    color: var(--accent);
  }
  .status {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 13px;
    color: var(--text-soft);
  }
  .status .done {
    color: var(--teal);
  }
  /* One line in compact rows: "42% read" then a short bar. */
  .compact .status {
    flex-direction: row-reverse;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
  }
  .compact .mini-bar {
    width: 36px;
  }
  .mini-bar {
    display: block;
    width: 80px;
    height: 4px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .mini-bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .where {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  .where.device {
    color: var(--teal);
  }
  .where.waiting {
    color: var(--accent);
  }

  .panel {
    position: sticky;
    /* Below the top bar. */
    top: 84px;
    max-height: calc(100vh - 120px);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 22px;
    border-radius: 24px;
    border: 0;
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--glass-shadow);
  }
  .cover-btn {
    align-self: center;
    width: 190px;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
    border-radius: var(--r1);
    box-shadow: var(--shadow-2);
    transition: transform var(--dur) var(--ease);
  }
  .cover-btn:hover {
    transform: translateY(-3px);
  }
  .heading {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .panel-series {
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent);
  }
  h2 {
    margin: 0;
    font-family: var(--font-serif);
    font-variant-numeric: lining-nums;
    font-size: 22px;
    line-height: 1.2;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .panel-author {
    margin: 0;
    color: var(--muted);
  }
  .state {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: var(--r2);
    background: var(--surface);
    border: 1px solid var(--border-soft);
  }
  .state-line {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 13px;
    font-weight: 600;
  }
  .state-line .device {
    color: var(--teal);
  }
  .state-line .muted {
    font-weight: 400;
  }
  .bar {
    height: 5px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .actions .wide {
    grid-column: span 2;
  }
  .actions .teal {
    background: var(--teal);
    border-color: var(--teal);
    color: var(--on-teal);
    font-weight: 700;
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border-soft);
  }
  dt {
    font-size: 11px;
    color: var(--faint);
  }
  dd {
    margin: 2px 0 0;
    font-size: 13px;
    word-break: break-word;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--surface-chip);
    color: var(--text-soft);
    font-size: 12px;
  }
  .chip.collection {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .description {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-body);
    overflow: hidden;
  }
</style>
