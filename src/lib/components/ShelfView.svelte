<script lang="ts">
  import type { Book } from '$lib/api';
  import { seriesLabel, statusLabel } from '$lib/format';
  import { bookActions } from '$lib/stores/bookActions.svelte';
  import { devices } from '$lib/stores/devices.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import { tip } from '$lib/tooltip';

  // The library as a bookcase: covers standing on wooden shelves, one plank per row.

  interface Props {
    books: Book[];
    onopen: (book: Book) => void;
  }

  let { books, onopen }: Props = $props();

  const BOOK_W = 128; // every book is the same size, in px
  const GAP = 16;
  const INSET = 28; // space between the case's sides and the books

  let width = $state(0);
  const perRow = $derived(Math.max(1, Math.floor((width - INSET * 2 + GAP) / (BOOK_W + GAP))));
  const rows = $derived.by(() => {
    const out: Book[][] = [];
    for (let i = 0; i < books.length; i += perRow) out.push(books.slice(i, i + perRow));
    return out;
  });


  function oncontextmenu(e: MouseEvent, book: Book) {
    e.preventDefault();
    let { clientX: x, clientY: y } = e;
    if (x === 0 && y === 0) {
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      x = r.left + 24;
      y = r.top + 24;
    }
    bookActions.openMenu(x, y, book, () => onopen(book));
  }
</script>

<div class="bookcase" bind:clientWidth={width} style:--inset="{INSET}px" style:--gap="{GAP}px">
  {#each rows as row, r (r)}
    <div class="shelf">
      <div class="books">
        {#each row as book (book.id)}
          {@const sync = devices.bookSync(book)}
          {@const series = seriesLabel(book)}
          <button
            class="book"
            style:width="{BOOK_W}px"
            onclick={() => onopen(book)}
            oncontextmenu={(e) => oncontextmenu(e, book)}
            aria-haspopup="menu"
            aria-label="{book.title} by {book.author}. {sync.label}, {statusLabel(book)}"
          >
            <span class="cover">
              <Cover title={book.title} author={book.author} coverPath={book.coverPath} reserveCorner />
              {#if sync.state === 'synced'}
                <span class="dot on-device" aria-hidden="true" use:tip={sync.tip}><Icon name="check" size={13} /></span>
              {:else if sync.state === 'waiting'}
                <span class="dot queued" aria-hidden="true" use:tip={sync.tip}><Icon name="sync" size={13} /></span>
              {/if}
              {#if book.status === 'reading'}
                <span class="progress" aria-hidden="true"><span style:width="{book.progress}%"></span></span>
              {:else if book.status === 'finished'}
                <span class="progress done" aria-hidden="true"><span style:width="100%"></span></span>
              {/if}
            </span>
            <span class="label" aria-hidden="true">
              <strong>{book.title}</strong>
              <span>{book.author}</span>
              {#if series}<span class="mono series">{series}</span>{/if}
              <span class="mono state">{sync.label} · {statusLabel(book)}</span>
            </span>
          </button>
        {/each}
      </div>
      <div class="plank" aria-hidden="true"></div>
    </div>
  {/each}
</div>

<style>
  /* The case: a dark wooden back wall with darker sides. */
  .bookcase {
    position: relative;
    padding: 6px var(--inset) 26px;
    border-radius: var(--r3);
    background:
      linear-gradient(90deg, var(--case-side), transparent 7%, transparent 93%, var(--case-side)),
      repeating-linear-gradient(90deg, var(--case-grain) 0 1px, transparent 1px 7px),
      linear-gradient(180deg, var(--case-wall-1) 0%, var(--case-wall-2) 100%);
    border: 1px solid var(--case-border);
    box-shadow:
      inset 0 24px 40px -24px var(--case-inset),
      0 18px 40px var(--case-drop);
  }

  .shelf {
    position: relative;
    padding-top: 34px;
  }
  /* Light falls from above, so the wall just above each plank is in shadow. */
  .shelf::before {
    content: '';
    position: absolute;
    left: calc(var(--inset) * -1);
    right: calc(var(--inset) * -1);
    bottom: 16px;
    height: 46px;
    background: linear-gradient(0deg, var(--shelf-shade), transparent);
    pointer-events: none;
  }
  .books {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: flex-end;
    gap: var(--gap);
    padding: 0 4px;
  }

  .plank {
    position: relative;
    height: 18px;
    margin: 0 calc(var(--inset) * -1);
    background:
      repeating-linear-gradient(90deg, var(--plank-grain-a) 0 2px, transparent 2px 11px, var(--plank-grain-b) 11px 13px, transparent 13px 23px),
      linear-gradient(180deg, var(--plank-1) 0%, var(--plank-2) 30%, var(--plank-3) 100%);
    box-shadow:
      inset 0 1px 0 var(--plank-hi),
      0 14px 22px -8px var(--plank-drop);
  }
  /* The plank's front edge. */
  .plank::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 6px;
    background: linear-gradient(180deg, var(--plank-edge-1), var(--plank-edge-2));
  }

  .book {
    position: relative;
    flex: none;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
    transition: transform var(--dur) var(--ease);
    transform-origin: bottom center;
  }
  .book:hover,
  .book:focus-visible {
    transform: translateY(-10px);
    z-index: 3;
  }
  .book:focus-visible {
    outline: none;
  }
  .book:focus-visible .cover {
    box-shadow:
      0 0 0 2px var(--accent),
      -6px 0 14px var(--book-cast-1);
  }
  .cover {
    position: relative;
    display: block;
    border-radius: 3px 5px 5px 3px;
    overflow: hidden;
    /* Shadow cast to the left onto the next book and down onto the plank. */
    box-shadow:
      -6px 0 14px var(--book-cast-1),
      0 3px 2px var(--book-cast-2);
  }
  /* A hint of the spine's curve along the left edge. */
  .cover::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 9px;
    background: linear-gradient(90deg, var(--spine-hi), transparent 40%, var(--cover-spine));
    pointer-events: none;
  }
  .cover :global(*) {
    border-radius: inherit;
  }

  .dot {
    position: absolute;
    top: 6px;
    right: 6px;
    z-index: 1;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.22);
  }
  /* Sync badges. Up to date is the normal case, so it's a quiet white dot; only a book
     waiting for the next sync gets a bold blue badge. */
  .on-device {
    background: rgba(255, 255, 255, 0.92);
    backdrop-filter: blur(8px);
    color: var(--teal);
  }
  .queued {
    background: var(--accent);
    color: var(--on-accent);
  }
  .progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1;
    height: 4px;
    background: var(--cover-track);
  }
  .progress span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .progress.done span {
    background: var(--teal);
  }

  /* Title card that appears above a book on hover or keyboard focus. */
  .label {
    position: absolute;
    left: 50%;
    bottom: calc(100% + 10px);
    z-index: 4;
    width: max-content;
    max-width: 240px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 9px 12px;
    border-radius: var(--r2);
    background: var(--surface-raised);
    border: 1px solid var(--border-control);
    box-shadow: var(--shadow-2);
    text-align: left;
    color: var(--text);
    opacity: 0;
    transform: translate(-50%, 6px);
    transition:
      opacity var(--dur) var(--ease),
      transform var(--dur) var(--ease);
    pointer-events: none;
  }
  .book:hover .label,
  .book:focus-visible .label {
    opacity: 1;
    transform: translate(-50%, 0);
  }
  .label strong {
    font-family: var(--font-serif);
    font-size: 14px;
    font-weight: 600;
    line-height: 1.25;
  }
  .label span {
    font-size: 12px;
    color: var(--muted);
  }
  .label .series {
    color: var(--accent);
    font-size: 11px;
  }
  .label .state {
    font-size: 11px;
    color: var(--faint);
  }

  @media (prefers-reduced-motion: reduce) {
    .book,
    .label {
      transition: none;
    }
  }
</style>
