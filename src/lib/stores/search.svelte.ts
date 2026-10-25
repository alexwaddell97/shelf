import { searchCatalog, type CatalogBook } from '$lib/api';
import { errorMessage } from '$lib/format';
import { nav } from './nav.svelte';

export type CatalogStatus = 'idle' | 'loading' | 'done' | 'error';

const DEBOUNCE_MS = 350;

/** Online catalogue lookup behind the header search box. Debounced, cached per query. */
class SearchStore {
  status = $state<CatalogStatus>('idle');
  results = $state<CatalogBook[]>([]);
  error = $state<string | null>(null);
  /** The query these results belong to. */
  resolvedFor = $state('');

  #timer: ReturnType<typeof setTimeout> | undefined;
  #seq = 0;
  #cache = new Map<string, CatalogBook[]>();

  #target = '';

  /**
   * Follows the header search box (nav.query). Safe to call repeatedly with the same text:
   * the search screen calls it from an effect whenever the query changes.
   */
  input(raw: string) {
    const q = raw.trim();
    if (q === this.#target && this.status !== 'idle') return;
    this.#target = q;
    clearTimeout(this.#timer);
    this.#seq++;
    if (!q) {
      this.status = 'idle';
      this.results = [];
      this.error = null;
      this.resolvedFor = '';
      return;
    }
    const cached = this.#cache.get(q.toLowerCase());
    if (cached) {
      this.results = cached;
      this.status = 'done';
      this.error = null;
      this.resolvedFor = q;
      return;
    }
    this.status = 'loading';
    this.error = null;
    this.#timer = setTimeout(() => void this.#run(q), DEBOUNCE_MS);
  }

  /** Re-runs the current query (the "Try again" button). */
  retry() {
    const q = nav.query.trim();
    if (!q) return;
    this.#cache.delete(q.toLowerCase());
    this.status = 'loading';
    this.error = null;
    clearTimeout(this.#timer);
    void this.#run(q);
  }

  async #run(q: string) {
    const seq = ++this.#seq;
    try {
      const results = await searchCatalog(q);
      if (seq !== this.#seq) return;
      this.#cache.set(q.toLowerCase(), results);
      this.results = results;
      this.resolvedFor = q;
      this.status = 'done';
    } catch (e) {
      if (seq !== this.#seq) return;
      console.error('Catalogue search failed', e);
      this.results = [];
      this.error = errorMessage(e);
      this.status = 'error';
    }
  }
}

export const search = new SearchStore();
