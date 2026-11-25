import { open } from '@tauri-apps/plugin-dialog';
import {
  forgetGoodreadsProfile,
  getGoodreads,
  importGoodreadsCsv,
  type GoodreadsApplied,
  type GoodreadsStatus
} from '$lib/api';
import { library } from './library.svelte';
import { attempt, toasts } from './toast.svelte';
import { wishlist } from './wishlist.svelte';

// Goodreads, by importing the library export. Goodreads retired the shelf feeds Shelf used
// to read (November 2026) and shelf pages need signing in, so there's no live connection.

class GoodreadsStore {
  status = $state<GoodreadsStatus | null>(null);
  importing = $state(false);

  async load() {
    const status = await attempt('Could not read your Goodreads import', getGoodreads);
    if (status) this.status = status;
  }

  /** Dismisses the note about the old profile link. */
  async forgetProfileLink() {
    const ok = await attempt('Could not remove the old Goodreads link', async () => {
      await forgetGoodreadsProfile();
      return true;
    });
    if (ok && this.status) this.status = { ...this.status, oldProfileLink: false };
  }

  /** File picker for the export CSV, then import. */
  async importCsv() {
    if (this.importing) return;
    const picked = await attempt('Could not open the file picker', () =>
      open({ multiple: false, filters: [{ name: 'Goodreads library export', extensions: ['csv'] }] })
    );
    if (!picked || Array.isArray(picked)) return;
    this.importing = true;
    try {
      const last = await attempt('Could not import that file', () => importGoodreadsCsv(picked));
      if (last) {
        this.status = { lastImport: last, oldProfileLink: this.status?.oldProfileLink ?? false };
        toasts.success(describeApplied(last.applied));
        void wishlist.refresh();
        void library.refresh();
      }
    } finally {
      this.importing = false;
    }
  }
}

export function describeApplied(a: GoodreadsApplied): string {
  const parts: string[] = [];
  if (a.addedToWishlist) parts.push(`${a.addedToWishlist} added to your wishlist`);
  if (a.removedFromWishlist) parts.push(`${a.removedFromWishlist} removed from it`);
  if (a.markedReading) parts.push(`${a.markedReading} marked as reading`);
  if (a.markedFinished) parts.push(`${a.markedFinished} marked as finished`);
  return parts.length ? `Goodreads import: ${parts.join(', ')}.` : 'Goodreads import finished. Shelf was already up to date.';
}

export const goodreads = new GoodreadsStore();
