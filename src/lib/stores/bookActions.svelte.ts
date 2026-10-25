// Actions on a single book, shared by the right-click menu (and anywhere else that
// needs them), plus the state of that menu and its delete confirmation.

import {
  deleteBook,
  openBook,
  queueForDevice,
  removeFromDevice,
  revealBook,
  unqueue,
  type Book,
  type Device
} from '$lib/api';
import { devices } from './devices.svelte';
import { library } from './library.svelte';
import { nav } from './nav.svelte';
import { attempt, toasts } from './toast.svelte';

export interface MenuState {
  x: number;
  y: number;
  book: Book;
  /** Opens this book's page the way its card does (keeps Back and Previous/Next working). */
  open: () => void;
}

async function run(label: string, fn: () => Promise<unknown>, success?: string) {
  const ok = await attempt(label, async () => {
    await fn();
    return true;
  });
  if (ok) {
    if (success) toasts.success(success);
    await library.refresh();
  }
}

class BookActions {
  menu = $state<MenuState | null>(null);
  /** The book waiting for delete confirmation. */
  deleting = $state<Book | null>(null);
  deleteFile = $state(false);
  busy = $state(false);

  openMenu(x: number, y: number, book: Book, open: () => void) {
    this.menu = { x, y, book, open };
  }

  closeMenu() {
    this.menu = null;
  }

  edit(book: Book, open: () => void) {
    open();
    // The editor opens over the page of the book being shown.
    if (nav.view.name === 'book' && nav.view.id === book.id) nav.editing = true;
  }

  send(book: Book, device: Device) {
    return run(
      `Could not queue the book for ${device.name}`,
      () => queueForDevice(device.id, [book.id]),
      `Queued. It goes to your ${device.name} on the next sync.`
    );
  }

  cancelSend(book: Book, deviceId: string) {
    return run('Could not cancel sending', () => unqueue(deviceId, book.id), 'Removed from the send queue.');
  }

  removeFromDevice(book: Book, device: Device) {
    return run(
      `Could not remove the book from ${device.name}`,
      () => removeFromDevice(device.id, book.id),
      `Removed from your ${device.name}. It is still in your library.`
    );
  }

  read(book: Book) {
    return run('Could not open the book', () => openBook(book.id));
  }

  reveal(book: Book) {
    return run('Could not show the file', () => revealBook(book.id));
  }

  askDelete(book: Book) {
    this.deleteFile = false;
    this.deleting = book;
  }

  cancelDelete() {
    if (!this.busy) this.deleting = null;
  }

  async confirmDelete() {
    const book = this.deleting;
    if (!book || this.busy) return;
    this.busy = true;
    const alsoFile = this.deleteFile;
    try {
      const ok = await attempt('Could not delete the book', async () => {
        await deleteBook(book.id, alsoFile);
        return true;
      });
      if (ok) {
        this.deleting = null;
        library.remove(book.id);
        toasts.success(`Removed “${book.title}” from your library${alsoFile ? ' and deleted its file' : ''}.`);
        if (nav.view.name === 'book' && nav.view.id === book.id) nav.go('library');
        void library.refresh();
        void devices.refresh();
      }
    } finally {
      this.busy = false;
    }
  }
}

export const bookActions = new BookActions();
