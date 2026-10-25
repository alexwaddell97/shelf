import { open } from '@tauri-apps/plugin-dialog';
import {
  checkGoodreads,
  connectGoodreads,
  disconnectGoodreads,
  getGoodreads,
  importGoodreadsCsv,
  type GoodreadsAccount,
  type GoodreadsApplied
} from '$lib/api';
import { errorMessage } from '$lib/format';
import { library } from './library.svelte';
import { attempt, toasts } from './toast.svelte';
import { wishlist } from './wishlist.svelte';

class GoodreadsStore {
  account = $state<GoodreadsAccount | null>(null);
  loaded = $state(false);
  /** Connecting and checking can take several seconds. */
  connecting = $state(false);
  checking = $state(false);
  importing = $state(false);
  /** Readable error from the last connect attempt, shown under the input. */
  error = $state<string | null>(null);

  async load() {
    const account = await attempt('Could not read your Goodreads connection', getGoodreads);
    if (account !== undefined) {
      this.account = account;
      this.loaded = true;
    }
  }

  async connect(profile: string): Promise<boolean> {
    if (this.connecting) return false;
    this.connecting = true;
    this.error = null;
    try {
      this.account = await connectGoodreads(profile.trim());
      toasts.success('Goodreads connected. Your Want to Read shelf is now in your wishlist.');
      void wishlist.refresh();
      void library.refresh();
      return true;
    } catch (e) {
      this.error = errorMessage(e);
      return false;
    } finally {
      this.connecting = false;
    }
  }

  async check() {
    if (this.checking) return;
    this.checking = true;
    try {
      const account = await attempt('Could not check your Goodreads shelves', checkGoodreads);
      if (account) {
        this.account = account;
        toasts.success(
          account.lastAdded > 0
            ? `Checked your shelves: ${account.lastAdded} new on your wishlist.`
            : 'Checked your Goodreads shelves. Nothing new.'
        );
        void wishlist.refresh();
        void library.refresh();
      }
    } finally {
      this.checking = false;
    }
  }

  async disconnect() {
    const ok = await attempt('Could not disconnect Goodreads', async () => {
      await disconnectGoodreads();
      return true;
    });
    if (ok) {
      this.account = null;
      this.error = null;
      toasts.info('Goodreads disconnected. Books already in your wishlist stay there.');
    }
  }

  /** File picker for the "Export library" CSV, then import. */
  async importCsv() {
    if (this.importing) return;
    const picked = await attempt('Could not open the file picker', () =>
      open({ multiple: false, filters: [{ name: 'Goodreads export', extensions: ['csv'] }] })
    );
    if (!picked || Array.isArray(picked)) return;
    this.importing = true;
    try {
      const applied = await attempt('Could not import that file', () => importGoodreadsCsv(picked));
      if (applied) {
        toasts.success(describeApplied(applied));
        void wishlist.refresh();
        void library.refresh();
      }
    } finally {
      this.importing = false;
    }
  }
}

function describeApplied(a: GoodreadsApplied): string {
  const parts: string[] = [];
  if (a.addedToWishlist) parts.push(`${a.addedToWishlist} added to your wishlist`);
  if (a.removedFromWishlist) parts.push(`${a.removedFromWishlist} removed from it`);
  if (a.markedReading) parts.push(`${a.markedReading} marked as reading`);
  if (a.markedFinished) parts.push(`${a.markedFinished} marked as finished`);
  return parts.length ? `Goodreads import: ${parts.join(', ')}.` : 'Goodreads import finished. Nothing needed changing.';
}

export const goodreads = new GoodreadsStore();
