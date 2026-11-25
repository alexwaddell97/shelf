import { fillMissingDetails, onDetailsProgress, type SyncProgress } from '$lib/api';
import { plural } from '$lib/format';
import { library } from './library.svelte';
import { attempt, toasts } from './toast.svelte';

// "Fill in missing details" for many books: one lookup at a time on Open Library (about a
// book a second), filling only empty fields and missing covers. Progress shows in a panel.

class DetailsStore {
  running = $state(false);
  progress = $state<SyncProgress | null>(null);

  async fill(ids: number[]) {
    if (this.running || ids.length === 0) return;
    this.running = true;
    this.progress = { deviceId: '', phase: 'details', done: 0, total: ids.length, current: null };
    const stop = await onDetailsProgress((p) => (this.progress = p)).catch(() => null);
    try {
      const report = await attempt('Could not fill in the details', () => fillMissingDetails(ids));
      if (report) {
        const parts = [`${plural(report.updated, 'book')} updated`];
        if (report.covers) parts.push(`${plural(report.covers, 'new cover')}`);
        if (report.notFound) parts.push(`${report.notFound} not found`);
        if (report.failed) parts.push(`${report.failed} couldn't be looked up`);
        toasts.success(`Details filled in: ${parts.join(', ')}.`);
      }
      await library.refresh();
    } finally {
      stop?.();
      this.running = false;
      this.progress = null;
    }
  }
}

export const details = new DetailsStore();
