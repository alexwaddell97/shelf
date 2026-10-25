<script lang="ts">
  import type { Book } from '$lib/api';
  import { seriesLabel, statusLabel } from '$lib/format';
  import { bookActions } from '$lib/stores/bookActions.svelte';
  import { devices } from '$lib/stores/devices.svelte';
  import { library } from '$lib/stores/library.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    book: Book;
    onopen: () => void;
    /** Show the progress bar, "New" badge and queue badge (the library grid does; search results stay lean). */
    detailed?: boolean;
  }

  let { book, onopen, detailed = true }: Props = $props();

  const onDevice = $derived(book.onDevices.length > 0);
  const queued = $derived(!onDevice && book.queuedFor.length > 0);
  const deviceTitle = $derived(book.onDevices.map((id) => devices.name(id)).join(', '));
  const isNew = $derived(detailed && library.fresh.includes(book.id));
  // Where the book is, then how far through it you are: "On Kobo · 42% read".
  const where = $derived(
    onDevice
      ? `On ${(devices.name(book.onDevices[0]) ?? 'device').split(' ')[0]}`
      : queued
        ? 'Waiting to send'
        : 'On PC only'
  );
  const meta = $derived(book.status === 'unread' && !onDevice ? where : `${where} · ${statusLabel(book)}`);
  const series = $derived(seriesLabel(book));

  function oncontextmenu(e: MouseEvent) {
    e.preventDefault();
    // The keyboard's menu key reports no pointer position: open by the card instead.
    let { clientX: x, clientY: y } = e;
    if (x === 0 && y === 0) {
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      x = r.left + 24;
      y = r.top + 24;
    }
    bookActions.openMenu(x, y, book, onopen);
  }
</script>

<button class="card" onclick={onopen} {oncontextmenu} aria-haspopup="menu" aria-label="{book.title} by {book.author}">
  <div class="lift">
    <Cover title={book.title} author={book.author} coverPath={book.coverPath} reserveCorner />
    <!-- Round status dots in the top-right corner, which Cover keeps free of text. -->
    <div class="badges">
      {#if isNew}
        <span class="badge new" role="img" aria-label="New" title="New">N</span>
      {/if}
      {#if onDevice}
        <span class="badge on-device" role="img" aria-label="On {deviceTitle}" title="On {deviceTitle}">
          <Icon name="device" size={13} />
        </span>
      {:else if queued && detailed}
        <span class="badge queued" role="img" aria-label="Waiting to send" title="Waiting to send">
          <Icon name="device-send" size={13} />
        </span>
      {/if}
    </div>
    {#if detailed && book.status === 'reading'}
      <span class="progress" aria-hidden="true"><span style:width="{book.progress}%"></span></span>
    {/if}
  </div>
  <span class="text">
    <span class="title">{book.title}</span>
    <span class="author">{book.author}</span>
    {#if series}<span class="meta series mono" title={series}>{series}</span>{/if}
    <span class="meta mono">{meta}</span>
  </span>
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
  }
  .lift {
    position: relative;
    width: 100%;
    transition: transform 0.18s ease;
  }
  .card:hover .lift,
  .card:focus-visible .lift {
    transform: translateY(-4px);
  }
  .badges {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--chip-overlay);
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.12);
    font-size: 11px;
    font-weight: 700;
  }
  .badge.on-device {
    color: var(--teal);
  }
  .badge.queued {
    color: var(--accent);
  }
  .badge.new {
    background: var(--accent);
    color: var(--on-accent);
    box-shadow: none;
  }
  .progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4px;
    overflow: hidden;
    border-radius: 0 0 var(--radius-s) var(--radius-s);
    background: rgba(0, 0, 0, 0.5);
  }
  .progress span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .title {
    font-weight: 600;
  }
  .title,
  .author {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .author {
    font-size: 13px;
    color: var(--muted);
  }
  .meta {
    font-size: 11px;
    color: var(--faint);
  }
  .meta.series {
    color: var(--accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  @media (prefers-reduced-motion: reduce) {
    .lift {
      transition: none;
    }
    .card:hover .lift,
    .card:focus-visible .lift {
      transform: none;
    }
  }
</style>
