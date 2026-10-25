import { open } from '@tauri-apps/plugin-dialog';
import { importFiles, type ImportProgress, type ImportStep } from '$lib/api';
import { basename, extension } from '$lib/format';
import { library } from './library.svelte';
import { attempt, toasts } from './toast.svelte';
import { wishlist } from './wishlist.svelte';

export const EBOOK_EXTENSIONS = ['epub', 'kepub', 'mobi', 'azw3', 'pdf'];

export interface ImportRow {
  file: string;
  name: string;
  step: ImportStep;
  matched: boolean;
  error: string | null;
  bookId: number | null;
}

const isTerminal = (s: ImportStep) => s === 'done' || s === 'failed';

class ImportsStore {
  rows = $state<ImportRow[]>([]);
  /** True while files are being dragged over the window. */
  dragging = $state(false);

  #clearTimer: ReturnType<typeof setTimeout> | undefined;

  get active(): number {
    return this.rows.filter((r) => !isTerminal(r.step)).length;
  }
  get failed(): number {
    return this.rows.filter((r) => r.step === 'failed').length;
  }
  get finished(): number {
    return this.rows.filter((r) => r.step === 'done').length;
  }

  onProgress(p: ImportProgress) {
    const row: ImportRow = {
      file: p.file,
      name: basename(p.file),
      step: p.step,
      matched: p.matchedWishlist,
      error: p.error,
      bookId: p.bookId
    };
    const i = this.rows.findIndex((r) => r.file === p.file);
    if (i === -1) {
      // A new batch replaces the leftovers of the last one.
      const keep = this.active > 0 ? this.rows : [];
      this.rows = [...keep, row];
    } else {
      // The wishlist note should stick even if a later event omits it.
      row.matched = row.matched || this.rows[i].matched;
      this.rows = this.rows.map((r, j) => (j === i ? row : r));
    }
    if (isTerminal(p.step)) {
      void library.refresh();
      if (p.matchedWishlist) void wishlist.refresh();
    }
    this.#scheduleClear();
  }

  dismiss() {
    clearTimeout(this.#clearTimer);
    this.rows = [];
  }

  #scheduleClear() {
    clearTimeout(this.#clearTimer);
    // Success fades away on its own; failures stay until the user closes the panel.
    if (this.rows.length > 0 && this.active === 0 && this.failed === 0) {
      this.#clearTimer = setTimeout(() => (this.rows = []), 6000);
    }
  }

  /** Imports files by absolute path. Progress comes back through `import-progress` events. */
  async importPaths(paths: string[]) {
    const supported = paths.filter((p) => EBOOK_EXTENSIONS.includes(extension(p)));
    if (supported.length === 0) {
      toasts.info('Shelf imports .epub, .kepub, .mobi, .azw3 and .pdf files.');
      return;
    }
    if (supported.length < paths.length) {
      toasts.info(`Skipped ${paths.length - supported.length} file(s) that are not ebooks.`);
    }
    const ids = await attempt('Could not import those files', () => importFiles(supported));
    if (ids) {
      void library.refresh();
      void wishlist.refresh();
    }
  }

  /** The "Add books" button. */
  async pickAndImport(): Promise<number> {
    const picked = await attempt('Could not open the file picker', () =>
      open({
        multiple: true,
        filters: [{ name: 'Ebooks', extensions: EBOOK_EXTENSIONS }]
      })
    );
    if (!picked) return 0;
    const paths = Array.isArray(picked) ? picked : [picked];
    if (paths.length === 0) return 0;
    await this.importPaths(paths);
    return paths.length;
  }
}

export const imports = new ImportsStore();
