// Following a book you went to get: after you open a store or source from a book's page,
// Shelf waits for the download to be imported and then shows it, wherever you are.

import { getBook, type CatalogBook } from '$lib/api';
import { sameWork } from '$lib/search';
import { library } from './library.svelte';
import { nav } from './nav.svelte';
import { toasts } from './toast.svelte';

export interface Expected {
  key: string;
  title: string;
  author: string;
  /** Where you went to get it, e.g. "Kobo Store". */
  source: string;
}

class ArrivalsStore {
  /** Books you've gone to get and that haven't arrived yet. */
  expected = $state<Expected[]>([]);

  expect(book: CatalogBook, source: string) {
    const item = { key: book.key, title: book.title, author: book.author, source };
    this.expected = [...this.expected.filter((e) => e.key !== book.key), item];
  }

  waitingFor(book: CatalogBook): Expected | undefined {
    return this.expected.find((e) => e.key === book.key);
  }

  stopWaiting(book: CatalogBook) {
    this.expected = this.expected.filter((e) => e.key !== book.key);
  }

  /** Called after a book has been imported. */
  async onImported(bookId: number) {
    // The library list may not have caught up yet (two refreshes can race, and the older
    // one is discarded), so ask for the book directly if it isn't there.
    const book = library.byId(bookId) ?? (await getBook(bookId).catch(() => null));
    if (!book) return;
    const view = nav.view;
    const onItsPage = view.name === 'catalog' && sameWork(view.book, book);
    const awaited = this.expected.find((e) => sameWork(e, book));
    if (!onItsPage && !awaited) return;
    this.expected = this.expected.filter((e) => !sameWork(e, book));

    if (onItsPage && view.name === 'catalog') {
      // You're looking at the book you went to get: show it in your library instead.
      nav.openBook(book.id, view.back);
      toasts.success(`“${book.title}” is in your library now`, 'Arrived from your downloads');
    } else {
      toasts.success(`“${book.title}” arrived in your library`, 'Imported from your downloads', {
        label: 'Open',
        run: () => nav.openBook(book.id, { kind: 'library' })
      });
    }
  }
}

export const arrivals = new ArrivalsStore();
