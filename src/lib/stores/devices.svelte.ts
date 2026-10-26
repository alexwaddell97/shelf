import { listDevices, refreshDevices, syncDevice, type Book, type Device, type SyncProgress, type SyncReport } from '$lib/api';
import { formatBytes, plural, timeAgo } from '$lib/format';
import { library } from './library.svelte';
import { attempt } from './toast.svelte';

/** Where a book stands with your e-readers, worded the same everywhere it's shown. */
export interface BookSync {
  /** synced: on the device and up to date; waiting: new or changed, goes next sync. */
  state: 'synced' | 'waiting' | 'pc';
  /** Short label, e.g. "On Kobo Libra 2". */
  label: string;
  /** Hover card text: a heading line, then detail lines. */
  tip: string;
}

class DevicesStore {
  list = $state<Device[]>([]);
  loaded = $state(false);
  selectedId = $state<string | null>(null);
  /** One line for the sync button and panel, so they always agree. */
  get stageLabel(): string {
    const p = this.progress;
    if (!p || p.phase === 'checking') return 'Checking what to send…';
    if (p.phase === 'reading') return 'Updating reading progress…';
    return `Sending ${Math.min(p.done + 1, p.total)} of ${p.total}`;
  }

  /** Live sync state; null when nothing is syncing. */
  progress = $state<SyncProgress | null>(null);
  /** Device id that just finished a sync, shown briefly as "All synced". */
  justSynced = $state<string | null>(null);
  /** The result of the last sync, shown in the sync panel until dismissed (or for a few seconds). */
  lastReport = $state<SyncReport | null>(null);

  #running = false;
  #seq = 0;
  #syncedTimer: ReturnType<typeof setTimeout> | undefined;
  #reportTimer: ReturnType<typeof setTimeout> | undefined;

  dismissReport() {
    clearTimeout(this.#reportTimer);
    this.lastReport = null;
  }

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

  /** How `book` stands with your e-readers. Pending changes win over "up to date". */
  bookSync(book: Pick<Book, 'onDevices' | 'queuedFor'>): BookSync {
    if (book.queuedFor.length) {
      const id = book.queuedFor[0];
      const name = this.name(id);
      const when = this.byId(id)?.connected ? 'when you next press Sync' : 'next time you plug it in and sync';
      return book.onDevices.includes(id)
        ? { state: 'waiting', label: 'Changes waiting to sync', tip: `Changes waiting to sync\nYour edits go to ${name} ${when}.` }
        : { state: 'waiting', label: 'Waiting to send', tip: `Waiting to sync\nIt goes to ${name} ${when}.` };
    }
    if (book.onDevices.length) {
      const name = book.onDevices.map((id) => this.name(id)).join(' and ');
      return {
        state: 'synced',
        label: `On ${this.name(book.onDevices[0])}`,
        tip: `On ${name} · up to date\nReading progress comes back each time you sync.`
      };
    }
    return { state: 'pc', label: 'On PC only', tip: 'Only on this PC\nRight-click it to send it to your e-reader.' };
  }

  /** Hover card for the Sync button: what's happening, or what pressing it will do. */
  syncTip(device: Device): string {
    const p = this.progress;
    if (this.syncing) {
      const detail = p?.phase === 'sending' && p.current ? `\n${p.current}` : '';
      return `${this.stageLabel}${detail}\nThis usually takes a few seconds.`;
    }
    const r = this.lastReport;
    if (r && r.deviceId === device.id) {
      const parts = [`${plural(r.sent, 'book')} sent`, `progress updated for ${r.progressUpdated}`];
      const errors = r.errors.length ? `\n${plural(r.errors.length, 'problem')}: see the sync panel` : '';
      return `Synced just now\n${parts.join(', ')}.${errors}`;
    }
    const last = device.lastSynced ? `Last synced ${timeAgo(device.lastSynced)}` : 'Not synced yet';
    const queue = this.queueCount;
    const action = queue
      ? `Click to send ${plural(queue, 'waiting book')} and bring reading progress back.`
      : `Everything's on your ${device.name}. Click to bring reading progress back.`;
    return `${last}\n${action}`;
  }

  /** Hover card for the device chip in the header. */
  deviceTip(device: Device | undefined): string {
    if (!device) return 'No e-reader connected\nPlug it in over USB, then tap Connect on its screen.';
    const onIt = library.books.filter((b) => b.onDevices.includes(device.id)).length;
    const free = device.freeBytes != null ? ` · ${formatBytes(device.freeBytes)} free` : '';
    return `${device.name} connected\n${plural(onIt, 'book')} on it${free}\nClick for device settings.`;
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
    // Progress events and the sync's own result travel separately, so a last event can
    // land after the sync has finished. Only a sync we're running shows progress;
    // otherwise a late event would leave the button stuck on "Syncing…".
    if (!this.#running) return;
    this.progress = p;
  }

  async sync(deviceId: string) {
    if (this.#running) return;
    this.#running = true;
    this.justSynced = null;
    this.dismissReport();
    this.progress = { deviceId, phase: 'checking', done: 0, total: 0, current: null };
    try {
      const report = await attempt('Sync failed', () => syncDevice(deviceId));
      if (!report) return;
      this.lastReport = report;
      // A clean result fades away on its own; problems stay until dismissed.
      if (!report.errors.length) this.#reportTimer = setTimeout(() => (this.lastReport = null), 8000);
      // The sync panel shows the result, so no toast here.
      if (!report.errors.length) {
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
