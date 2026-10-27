import { listBooks, type Book, type ReadStatus } from '$lib/api';
import { authorEntries, authorSortKey, sortTitle } from '$lib/format';
import { labels } from './labels.svelte';
import { attempt } from './toast.svelte';

export type LibraryFilter = 'all' | ReadStatus | 'on-device' | 'not-on-device';
export type LibrarySort = 'recent' | 'title' | 'author' | 'series' | 'published' | 'rating';

export const SORT_OPTIONS: [LibrarySort, string][] = [
  ['recent', 'Recently added'],
  ['title', 'Title'],
  ['author', 'Author'],
  ['series', 'Series'],
  ['published', 'Published'],
  ['rating', 'Rating']
];

/** A filtered view of the library: one author, series, tag or collection. */
export interface Scope {
  kind: 'author' | 'series' | 'tag' | 'collection';
  key: string;
  /** Heading, e.g. "Series: The Expanse". */
  title: string;
}

const SORT_KEY = 'shelf.sort';
const VIEW_KEY = 'shelf.view';

/** How the library is laid out: covers on wooden shelves, a plain grid, or a list with a details panel. */
export type LibraryView = 'shelf' | 'grid' | 'list';

function loadView(): LibraryView {
  try {
    const v = localStorage.getItem(VIEW_KEY);
    if (v === 'shelf' || v === 'grid' || v === 'list') return v;
  } catch {
    /* storage unavailable */
  }
  return 'shelf';
}

function loadSort(): LibrarySort {
  try {
    const v = localStorage.getItem(SORT_KEY);
    if (v && SORT_OPTIONS.some(([id]) => id === v)) return v as LibrarySort;
  } catch {
    /* storage unavailable */
  }
  return 'recent';
}

const byTitle = (a: Book, b: Book) => sortTitle(a.title).localeCompare(sortTitle(b.title));
const byRecent = (a: Book, b: Book) => b.addedAt.localeCompare(a.addedAt) || b.id - a.id;

export function compareBooks(sort: LibrarySort): (a: Book, b: Book) => number {
  switch (sort) {
    case 'title':
      return (a, b) => byTitle(a, b) || byRecent(a, b);
    case 'author':
      return (a, b) => authorSortKey(a).localeCompare(authorSortKey(b)) || byTitle(a, b);
    case 'series':
      return (a, b) => {
        if (!a.series !== !b.series) return a.series ? -1 : 1; // books without a series last
        if (a.series && b.series) {
          const s = sortTitle(a.series).localeCompare(sortTitle(b.series));
          if (s) return s;
          const ai = a.seriesIndex ?? Number.POSITIVE_INFINITY;
          const bi = b.seriesIndex ?? Number.POSITIVE_INFINITY;
          if (ai !== bi) return ai < bi ? -1 : 1;
        }
        return byTitle(a, b);
      };
    case 'published':
      return (a, b) => {
        if (!a.published !== !b.published) return a.published ? -1 : 1;
        return (b.published ?? '').localeCompare(a.published ?? '') || byTitle(a, b);
      };
    case 'rating':
      return (a, b) => (b.rating ?? 0) - (a.rating ?? 0) || byTitle(a, b);
    default:
      return byRecent;
  }
}

class LibraryStore {
  books = $state<Book[]>([]);
  loaded = $state(false);
  filter = $state<LibraryFilter>('all');
  sort = $state<LibrarySort>(loadSort());
  view = $state<LibraryView>(loadView());
  /** Ids that showed up after the first load, so the grid can mark them "New". */
  fresh = $state<Set<number>>(new Set());


  setView(view: LibraryView) {
    this.view = view;
    try {
      localStorage.setItem(VIEW_KEY, view);
    } catch {
      /* storage unavailable */
    }
  }

  setSort(sort: LibrarySort) {
    this.sort = sort;
    try {
      localStorage.setItem(SORT_KEY, sort);
    } catch {
      /* storage unavailable */
    }
  }

  #loading: Promise<void> | null = null;
  #queued: Promise<void> | null = null;
  /** Bumped by local edits, so a load that started before one isn't applied over it. */
  #edits = 0;

  /**
   * Reloads the library. Calls while a load is running share one follow-up load, so a
   * batch of imports doesn't fire a load per file, and awaiting always means "a load that
   * started after I asked has finished".
   */
  refresh(): Promise<void> {
    if (!this.#loading) {
      this.#loading = this.#load().finally(() => (this.#loading = null));
      return this.#loading;
    }
    this.#queued ??= this.#loading.then(() => {
      this.#queued = null;
      return this.refresh();
    });
    return this.#queued;
  }

  async #load() {
    const edits = this.#edits;
    const books = await attempt('Could not load your library', listBooks);
    if (!books) return;
    if (edits !== this.#edits) {
      // Edited while loading: this list may be older than the edit, so load again.
      void this.refresh();
      return;
    }
    if (this.loaded) {
      const known = new Set(this.books.map((b) => b.id));
      const added = books.filter((b) => !known.has(b.id)).map((b) => b.id);
      if (added.length) this.fresh = new Set([...this.fresh, ...added]);
    }
    this.books = books;
    this.loaded = true;
    void labels.refresh();
  }

  byId(id: number): Book | undefined {
    return this.books.find((b) => b.id === id);
  }

  upsert(book: Book) {
    this.#edits++;
    const i = this.books.findIndex((b) => b.id === book.id);
    this.books = i === -1 ? [book, ...this.books] : this.books.map((b) => (b.id === book.id ? book : b));
  }

  remove(id: number) {
    this.books = this.books.filter((b) => b.id !== id);
  }

  onDevice(deviceId: string): Book[] {
    return this.books.filter((b) => b.onDevices.includes(deviceId));
  }

  queuedFor(deviceId: string): Book[] {
    return this.books.filter((b) => b.queuedFor.includes(deviceId));
  }

  inScope(book: Book, scope?: Scope): boolean {
    if (!scope) return true;
    switch (scope.kind) {
      case 'author':
        return authorEntries(book).some((a) => a.key === scope.key);
      case 'series':
        return book.series === scope.key;
      case 'tag':
        return book.tags.includes(scope.key);
      default:
        return book.collections.includes(scope.key);
    }
  }

  matches(book: Book, filter: LibraryFilter): boolean {
    switch (filter) {
      case 'all':
        return true;
      case 'on-device':
        return book.onDevices.length > 0;
      case 'not-on-device':
        return book.onDevices.length === 0;
      default:
        return book.status === filter;
    }
  }

  count(filter: LibraryFilter, scope?: Scope): number {
    return this.books.filter((b) => this.inScope(b, scope) && this.matches(b, filter)).length;
  }

  /** Books in the scope, filtered by the chip and ordered by `sort`. */
  list(scope?: Scope, sort: LibrarySort = this.sort): Book[] {
    return this.books.filter((b) => this.inScope(b, scope) && this.matches(b, this.filter)).sort(compareBooks(sort));
  }

  get onDeviceCount(): number {
    return this.books.filter((b) => b.onDevices.length > 0).length;
  }
}

export const library = new LibraryStore();
