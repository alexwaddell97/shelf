import { listBooks, type Book, type ReadStatus } from '$lib/api';
import { authorEntries, authorSortKey, languageName, sortTitle } from '$lib/format';
import { labels } from './labels.svelte';
import { attempt } from './toast.svelte';

export type LibraryFilter = 'all' | ReadStatus | 'on-device' | 'not-on-device';
export type LibrarySort =
  | 'recent'
  | 'title'
  | 'author'
  | 'series'
  | 'published'
  | 'rating'
  | 'status'
  | 'device'
  | 'lastRead'
  | 'format'
  | 'size'
  | 'language';

interface SortDef {
  label: string;
  /** The order you get first: A to Z, or newest, biggest or best first. */
  first: 'asc' | 'desc';
  /** Compares two books in the `first` order. */
  cmp: (a: Book, b: Book) => number;
  /** Books without a value go last, whichever way round the list is. */
  has?: (b: Book) => boolean;
}

const byTitle = (a: Book, b: Book) => sortTitle(a.title).localeCompare(sortTitle(b.title));
const statusRank = (b: Book) => (b.status === 'finished' ? 200 : b.status === 'reading' ? 100 + b.progress : 0);
const deviceRank = (b: Book) => (b.queuedFor.length ? 1 : b.onDevices.length ? 2 : 0);

const SORTS: Record<LibrarySort, SortDef> = {
  recent: { label: 'Recently added', first: 'desc', cmp: (a, b) => b.addedAt.localeCompare(a.addedAt) || b.id - a.id },
  title: { label: 'Title', first: 'asc', cmp: byTitle },
  author: { label: 'Author', first: 'asc', cmp: (a, b) => authorSortKey(a).localeCompare(authorSortKey(b)) },
  series: {
    label: 'Series',
    first: 'asc',
    has: (b) => !!b.series,
    cmp: (a, b) =>
      sortTitle(a.series ?? '').localeCompare(sortTitle(b.series ?? '')) ||
      // Unnumbered books after numbered ones.
      (a.seriesIndex ?? 1e9) - (b.seriesIndex ?? 1e9)
  },
  published: { label: 'Published', first: 'desc', has: (b) => !!b.published, cmp: (a, b) => (b.published ?? '').localeCompare(a.published ?? '') },
  rating: { label: 'Rating', first: 'desc', has: (b) => !!b.rating, cmp: (a, b) => (b.rating ?? 0) - (a.rating ?? 0) },
  status: { label: 'Reading progress', first: 'desc', cmp: (a, b) => statusRank(b) - statusRank(a) },
  device: { label: 'On a device', first: 'desc', cmp: (a, b) => deviceRank(b) - deviceRank(a) },
  lastRead: { label: 'Last read', first: 'desc', has: (b) => !!b.lastRead, cmp: (a, b) => (b.lastRead ?? '').localeCompare(a.lastRead ?? '') },
  format: { label: 'Format', first: 'asc', cmp: (a, b) => a.format.localeCompare(b.format) },
  size: { label: 'File size', first: 'desc', cmp: (a, b) => b.sizeBytes - a.sizeBytes },
  language: {
    label: 'Language',
    first: 'asc',
    has: (b) => !!languageName(b.language),
    cmp: (a, b) => (languageName(a.language) ?? '').localeCompare(languageName(b.language) ?? '')
  }
};

export const SORT_OPTIONS = (Object.keys(SORTS) as LibrarySort[]).map((id): [LibrarySort, string] => [id, SORTS[id].label]);

/** True when the list runs A to Z, oldest first or smallest first. */
export const isAscending = (sort: LibrarySort, reversed: boolean) => (SORTS[sort].first === 'asc') !== reversed;

/** Orders books by `sort`, flipped when `reversed`. Ties go by title. */
export function compareBooks(sort: LibrarySort, reversed = false): (a: Book, b: Book) => number {
  const { cmp, has } = SORTS[sort] ?? SORTS.recent;
  const dir = reversed ? -1 : 1;
  return (a, b) => {
    if (has && has(a) !== has(b)) return has(a) ? -1 : 1;
    return dir * cmp(a, b) || byTitle(a, b) || a.id - b.id;
  };
}

/** A filtered view of the library: one author, series, tag or collection. */
export interface Scope {
  kind: 'author' | 'series' | 'tag' | 'collection';
  key: string;
  /** Heading, e.g. "Series: The Expanse". */
  title: string;
}

const SORT_KEY = 'shelf.sort';
const REVERSED_KEY = 'shelf.sortReversed';
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
    if (v && v in SORTS) return v as LibrarySort;
  } catch {
    /* storage unavailable */
  }
  return 'recent';
}

function loadReversed(): boolean {
  try {
    return localStorage.getItem(REVERSED_KEY) === '1';
  } catch {
    return false;
  }
}

class LibraryStore {
  books = $state<Book[]>([]);
  loaded = $state(false);
  filter = $state<LibraryFilter>('all');
  sort = $state<LibrarySort>(loadSort());
  /** Whether the sort runs the other way round (Z to A, oldest first). */
  reversed = $state(loadReversed());
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

  setSort(sort: LibrarySort, reversed = false) {
    this.sort = sort;
    this.reversed = reversed;
    try {
      localStorage.setItem(SORT_KEY, sort);
      localStorage.setItem(REVERSED_KEY, reversed ? '1' : '0');
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
  list(scope?: Scope, sort: LibrarySort = this.sort, reversed = this.reversed): Book[] {
    return this.books.filter((b) => this.inScope(b, scope) && this.matches(b, this.filter)).sort(compareBooks(sort, reversed));
  }

  get onDeviceCount(): number {
    return this.books.filter((b) => b.onDevices.length > 0).length;
  }
}

export const library = new LibraryStore();
