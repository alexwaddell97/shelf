import { listDevices, refreshDevices, syncDevice, type Device, type SyncProgress } from '$lib/api';
import { plural } from '$lib/format';
import { library } from './library.svelte';
import { attempt, toasts } from './toast.svelte';

class DevicesStore {
  list = $state<Device[]>([]);
  loaded = $state(false);
  selectedId = $state<string | null>(null);
  /** Live sync state; null when nothing is syncing. */
  progress = $state<SyncProgress | null>(null);
  /** Device id that just finished a sync, shown briefly as "All synced". */
  justSynced = $state<string | null>(null);

  #running = false;
  #seq = 0;
  #clearTimer: ReturnType<typeof setTimeout> | undefined;
  #syncedTimer: ReturnType<typeof setTimeout> | undefined;

  get connected(): Device[] {
    return this.list.filter((d) => d.connected);
  }

  get selected(): Device | undefined {
    return this.list.find((d) => d.id === this.selectedId) ?? this.connected[0] ?? this.list[0];
  }

  byId(id: string): Device | undefined {
    return this.list.find((d) => d.id === id);
  }

  name(id: string): string {
    return this.byId(id)?.name ?? 'device';
  }

  get syncing(): boolean {
    return this.#running || this.progress !== null;
  }

  /** Books waiting to go to any connected device. */
  get queueCount(): number {
    const ids = new Set(this.connected.map((d) => d.id));
    return library.books.filter((b) => b.queuedFor.some((id) => ids.has(id))).length;
  }

  /** `force` asks the backend to rescan the drives instead of returning what it last saw. */
  async refresh(force = false) {
    const seq = ++this.#seq;
    const list = await attempt('Could not check for e-readers', force ? refreshDevices : listDevices);
    if (!list || seq !== this.#seq) return;
    this.list = list;
    this.loaded = true;
    if (!this.selectedId || !list.some((d) => d.id === this.selectedId)) {
      this.selectedId = (list.find((d) => d.connected) ?? list[0])?.id ?? null;
    }
  }

  select(id: string) {
    this.selectedId = id;
  }

  onProgress(p: SyncProgress) {
    this.progress = p;
    clearTimeout(this.#clearTimer);
    // A sync we didn't start (auto-sync) never calls back into us, so let the bar expire.
    if (!this.#running && p.total > 0 && p.done >= p.total) {
      this.#clearTimer = setTimeout(() => (this.progress = null), 1500);
    }
  }

  async sync(deviceId: string) {
    if (this.#running) return;
    this.#running = true;
    clearTimeout(this.#clearTimer);
    this.justSynced = null;
    this.progress = { deviceId, done: 0, total: 0, current: null };
    try {
      const report = await attempt('Sync failed', () => syncDevice(deviceId));
      if (!report) return;
      const name = this.name(deviceId);
      const parts: string[] = [];
      if (report.sent) parts.push(`${plural(report.sent, 'book')} sent to your ${name}`);
      if (report.progressUpdated) parts.push(`reading progress updated for ${plural(report.progressUpdated, 'book')}`);
      if (report.errors.length) {
        const shown = report.errors.slice(0, 3).join('\n');
        const more = report.errors.length > 3 ? `\n…and ${report.errors.length - 3} more` : '';
        toasts.error(
          `Sync finished with ${plural(report.errors.length, 'problem')}`,
          shown + more
        );
      }
      if (!report.errors.length || report.sent || report.progressUpdated) {
        const text = parts.length ? parts.join(', ') : 'Everything is already up to date';
        toasts.success(text.charAt(0).toUpperCase() + text.slice(1) + '.');
        this.justSynced = deviceId;
        clearTimeout(this.#syncedTimer);
        this.#syncedTimer = setTimeout(() => (this.justSynced = null), 2600);
      }
    } finally {
      this.#running = false;
      this.progress = null;
      void library.refresh();
      void this.refresh();
    }
  }
}

export const devices = new DevicesStore();
