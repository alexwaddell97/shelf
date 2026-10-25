import { addToWishlist, listWishlist, removeFromWishlist, type CatalogBook, type WishItem } from '$lib/api';
import { looseKey } from '$lib/format';
import { attempt, toasts } from './toast.svelte';

class WishlistStore {
  items = $state<WishItem[]>([]);
  loaded = $state(false);
  #seq = 0;

  async refresh() {
    const seq = ++this.#seq;
    const items = await attempt('Could not load your wishlist', listWishlist);
    if (!items || seq !== this.#seq) return;
    this.items = items;
    this.loaded = true;
  }

  /** The wishlist entry for a catalogue book, matched by catalogue key, else by title and author. */
  find(book: Pick<CatalogBook, 'key' | 'title' | 'author'>): WishItem | undefined {
    if (book.key) {
      const byKey = this.items.find((w) => w.catalogKey === book.key);
      if (byKey) return byKey;
    }
    const k = looseKey(book.title, book.author);
    return this.items.find((w) => looseKey(w.title, w.author) === k);
  }

  async add(book: CatalogBook) {
    const item = await attempt('Could not add to your wishlist', () => addToWishlist(book));
    if (!item) return;
    this.items = [item, ...this.items.filter((w) => w.id !== item.id)];
    toasts.success('Added to your wishlist. Shelf matches the file when it arrives.');
  }

  async remove(id: number) {
    const ok = await attempt('Could not remove it from your wishlist', async () => {
      await removeFromWishlist(id);
      return true;
    });
    if (ok) this.items = this.items.filter((w) => w.id !== id);
  }
}

export const wishlist = new WishlistStore();
