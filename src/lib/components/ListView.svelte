<script lang="ts">
  import { tick } from 'svelte';
  import type { Book } from '$lib/api';
  import { formatBytes, formatDate, seriesLabel, statusLabel } from '$lib/format';
  import { bookActions } from '$lib/stores/bookActions.svelte';
  import { devices } from '$lib/stores/devices.svelte';
  import ClampText from './ClampText.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import Stars from './Stars.svelte';
  import { tip } from '$lib/tooltip';

  // The library as a list, with the selected book's cover and details in a side panel.
  // Click selects, double-click or Enter opens, arrow keys move.

  interface Props {
    books: Book[];
    onopen: (book: Book) => void;
  }

  let { books, onopen }: Props = $props();

  let selectedId = $state<number | null>(null);
  const selected = $derived(books.find((b) => b.id === selectedId) ?? books[0] ?? null);
  let listEl = $state<HTMLDivElement | null>(null);


  async function select(book: Book, scroll = false) {
    selectedId = book.id;
    if (!scroll) return;
    await tick();
    listEl?.querySelector<HTMLElement>(`[data-id="${book.id}"]`)?.scrollIntoView({ block: 'nearest' });
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
    }
  }

  function oncontextmenu(e: MouseEvent, book: Book) {
    e.preventDefault();
    void select(book);
    bookActions.openMenu(e.clientX, e.clientY, book, () => onopen(book));
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

<div class="layout">
  <div
    class="list"
    role="listbox"
    tabindex="0"
    aria-label="Books"
    aria-activedescendant={selected ? `book-row-${selected.id}` : undefined}
    bind:this={listEl}
    {onkeydown}
  >
    <div class="row head" aria-hidden="true">
      <span></span>
      <span>Title</span>
      <span class="col-series">Series</span>
      <span>Status</span>
      <span class="col-where">Where</span>
      <span class="col-added">Added</span>
    </div>
    {#each books as book (book.id)}
      {@const sync = devices.bookSync(book)}
      {@const on = selected?.id === book.id}
      {@const series = seriesLabel(book)}
      <div
        id="book-row-{book.id}"
        data-id={book.id}
        class="row"
        class:on
        role="option"
        aria-selected={on}
        tabindex="-1"
        onclick={() => select(book)}
        ondblclick={() => onopen(book)}
        oncontextmenu={(e) => oncontextmenu(e, book)}
        onkeydown={() => {}}
      >
        <span class="thumb"><Cover title={book.title} author={book.author} coverPath={book.coverPath} /></span>
        <span class="name">
          <span class="title">{book.title}</span>
          <span class="author">{book.author}</span>
        </span>
        <span class="col-series mono series" title={series}>{series}</span>
        <span class="status">
          {#if book.status === 'reading'}
            <span class="mini-bar" aria-hidden="true"><span style:width="{book.progress}%"></span></span>
          {/if}
          <span class:done={book.status === 'finished'}>{statusLabel(book)}</span>
        </span>
        <span class="col-where where" class:device={sync.state === 'synced'} class:waiting={sync.state === 'waiting'} use:tip={sync.tip}>
          {#if sync.state !== 'pc'}<Icon name={sync.state === 'synced' ? 'check' : 'sync'} size={13} />{/if}
          {sync.label}
        </span>
        <span class="col-added mono added">{formatDate(book.addedAt)}</span>
      </div>
    {/each}
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
        {#if book.language}<div><dt>Language</dt><dd>{book.language}</dd></div>{/if}
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

  .list {
    border-radius: 20px;
    border: 0;
    background: var(--glass-tint);
    box-shadow: var(--glass-shine), 0 0 0 0.5px var(--border-soft);
    overflow: hidden;
    outline: none;
  }
  .list:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .row {
    display: grid;
    grid-template-columns: 40px minmax(0, 2.2fr) minmax(0, 1.3fr) 130px 150px 96px;
    align-items: center;
    gap: 14px;
    min-height: 64px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border-faint);
    cursor: default;
    user-select: none;
  }
  .row:last-child {
    border-bottom: 0;
  }
  .row:not(.head):hover {
    background: var(--surface-hover);
  }
  .row.on {
    background: var(--accent-bg-strong);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .row.head {
    min-height: 38px;
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--surface-raised);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--faint);
  }
  @media (max-width: 1320px) {
    .row {
      grid-template-columns: 40px minmax(0, 2.2fr) minmax(0, 1.3fr) 130px 150px;
    }
    .col-added {
      display: none;
    }
  }
  @media (max-width: 900px) {
    .row {
      grid-template-columns: 40px minmax(0, 1fr) 120px;
    }
    .col-series,
    .col-where {
      display: none !important;
    }
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
  .title,
  .author,
  .series,
  .where {
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
  .series {
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
    font-size: 13px;
    color: var(--muted);
  }
  .where.device {
    color: var(--teal);
  }
  .where.waiting {
    color: var(--accent);
  }
  .added {
    font-size: 12px;
    color: var(--faint);
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
