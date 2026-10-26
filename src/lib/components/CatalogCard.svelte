<script lang="ts">
  import type { CatalogBook } from '$lib/api';
  import { wishlist } from '$lib/stores/wishlist.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    book: CatalogBook;
    onopen: () => void;
  }

  let { book, onopen }: Props = $props();

  const onWish = $derived(!!wishlist.find(book));
  const meta = $derived(
    book.isFree ? 'Free · public domain' : onWish ? 'On your wishlist' : book.year ? String(book.year) : 'Available to buy'
  );
</script>

<button class="card" onclick={onopen} aria-label="{book.title} by {book.author}, not in your library">
  <div class="lift">
    <Cover title={book.title} author={book.author} coverUrl={book.coverUrl} reserveCorner />
    {#if book.isFree}
      <span class="badge free" role="img" aria-label="Free to download" title="Free to download"><Icon name="drop" size={13} /></span>
    {:else if onWish}
      <span class="badge wish" role="img" aria-label="On your wishlist" title="On your wishlist"><Icon name="bookmark" size={13} /></span>
    {/if}
  </div>
  <span class="text">
    <span class="title">{book.title}</span>
    <span class="author">{book.author}</span>
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
    transition: transform var(--dur) var(--ease);
  }
  .card:hover .lift,
  .card:focus-visible .lift {
    transform: translateY(-4px);
  }
  .badge {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--chip-overlay);
    box-shadow: 0 0 0 1px var(--badge-ring);
  }
  .badge.free {
    color: var(--teal);
  }
  .badge.wish {
    color: var(--accent);
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
