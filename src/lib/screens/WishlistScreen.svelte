<script lang="ts">
  import Cover from '$lib/components/Cover.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { wishToCatalog } from '$lib/format';
  import { nav } from '$lib/stores/nav.svelte';
  import { wishlist } from '$lib/stores/wishlist.svelte';

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
    <div class="book-grid">
      {#each wishlist.items as item (item.id)}
        <div class="tile">
          <div class="ghost-frame">
            <Cover title={item.title} author={item.author} coverUrl={item.coverUrl} ghost reserveCorner />
            <span class="wanted" role="img" aria-label="Wanted" title="Wanted"><Icon name="bookmark" size={13} /></span>
          </div>
          <div class="text">
            <div class="title">{item.title}</div>
            <div class="author">{item.author}</div>
            {#if originLabel(item.origin)}
              <div class="from mono">{originLabel(item.origin)}</div>
            {/if}
          </div>
          <div class="buttons">
            <button class="btn sm find" onclick={() => nav.openCatalog(wishToCatalog(item), { kind: 'wishlist' })}>
              Where to get it
            </button>
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
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.12);
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
