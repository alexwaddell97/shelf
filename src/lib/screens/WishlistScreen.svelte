<script lang="ts">
  import Cover from '$lib/components/Cover.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { wishToCatalog } from '$lib/format';
  import type { Book } from '$lib/api';
  import { sameWork } from '$lib/search';
  import { library } from '$lib/stores/library.svelte';
  import { nav } from '$lib/stores/nav.svelte';
  import { toasts } from '$lib/stores/toast.svelte';
  import { wishlist } from '$lib/stores/wishlist.svelte';

  // Wishes are matched to the library when a book is imported. Books added before a match
  // was possible (or under a slightly different title) are caught here instead.
  const owned = $derived.by(() => {
    const found = new Map<number, Book>();
    for (const item of wishlist.items) {
      const book = library.books.find((b) => sameWork(b, item));
      if (book) found.set(item.id, book);
    }
    return found;
  });

  let clearing = $state(false);
  async function clearOwned() {
    if (clearing) return;
    clearing = true;
    const ids = [...owned.keys()];
    try {
      for (const id of ids) await wishlist.remove(id);
      toasts.success(`Removed ${ids.length === 1 ? '1 book' : `${ids.length} books`} you already have`);
    } finally {
      clearing = false;
    }
  }

  const originLabel = (origin: string) =>
    origin === 'search' ? '' : `From ${origin.charAt(0).toUpperCase()}${origin.slice(1)}`;
</script>

<section aria-labelledby="wish-h">
  <div>
    <h1 id="wish-h" class="page-title">Wishlist</h1>
    <p class="page-sub">
      Books you want but don't have yet. When a matching file lands in a watched folder or you add it yourself, its tile turns into a real book.
    </p>
  </div>

  {#if !wishlist.loaded}
    <p class="muted">Loading your wishlist…</p>
  {:else if wishlist.items.length === 0}
    <p class="empty">Your wishlist is empty. Search for a book and add it from “Not in your library”.</p>
  {:else}
    {#if owned.size}
      <div class="owned-banner">
        <Icon name="check" size={16} />
        <span>
          {owned.size === 1 ? '1 book on your wishlist is' : `${owned.size} books on your wishlist are`} already in your library.
        </span>
        <button class="btn sm" onclick={clearOwned} disabled={clearing}>Remove {owned.size === 1 ? 'it' : 'them'} from wishlist</button>
      </div>
    {/if}
    <div class="book-grid">
      {#each wishlist.items as item (item.id)}
        {@const have = owned.get(item.id)}
        <div class="tile">
          <div class="ghost-frame">
            <Cover title={item.title} author={item.author} coverUrl={item.coverUrl} ghost reserveCorner />
            {#if have}
              <span class="wanted have" role="img" aria-label="Already in your library" title="Already in your library"><Icon name="check" size={13} /></span>
            {:else}
              <span class="wanted" role="img" aria-label="Wanted" title="Wanted"><Icon name="bookmark" size={13} /></span>
            {/if}
          </div>
          <div class="text">
            <div class="title">{item.title}</div>
            <div class="author">{item.author}</div>
            {#if originLabel(item.origin)}
              <div class="from mono">{originLabel(item.origin)}</div>
            {/if}
          </div>
          <div class="buttons">
            {#if have}
              <button class="btn sm find have" onclick={() => nav.openBook(have.id, { kind: 'wishlist' })}>
                Open in library
              </button>
            {:else}
              <button class="btn sm find" onclick={() => nav.openCatalog(wishToCatalog(item), { kind: 'wishlist' })}>
                Where to get it
              </button>
            {/if}
            <button
              class="icon-btn"
              aria-label="Remove {item.title} from wishlist"
              title="Remove from wishlist"
              onclick={() => wishlist.remove(item.id)}
            >
              <Icon name="close" size={16} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .ghost-frame {
    position: relative;
    border-radius: var(--radius-m);
    outline: 1.5px dashed var(--border-dashed);
    outline-offset: 5px;
  }
  .wanted {
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
    color: var(--text);
    box-shadow: 0 0 0 1px var(--badge-ring);
  }
  .wanted.have {
    background: var(--teal);
    color: var(--on-teal);
    box-shadow: none;
  }
  .owned-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px 10px 14px;
    border-radius: var(--radius-m);
    background: var(--teal-bg);
    border: 1px solid var(--teal-border);
    color: var(--text);
    font-size: 14px;
  }
  .owned-banner :global(svg) {
    color: var(--teal);
    flex: none;
  }
  .owned-banner span {
    flex: 1;
  }
  .find.have {
    color: var(--teal);
  }
  .text {
    min-width: 0;
    margin-top: 4px;
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
  .from {
    font-size: 11px;
    color: var(--faint);
  }
  .buttons {
    display: flex;
    gap: 6px;
    margin-top: auto;
  }
  .find {
    flex: 1;
    height: 36px;
    padding: 0 8px;
    border-radius: var(--radius-m);
  }
</style>
