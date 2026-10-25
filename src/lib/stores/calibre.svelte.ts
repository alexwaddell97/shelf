import { open } from '@tauri-apps/plugin-dialog';
import {
  findCalibreLibrary,
  importCalibre,
  inspectCalibre,
  type CalibreProgress,
  type CalibreReport,
  type CalibreSummary
} from '$lib/api';
import { plural } from '$lib/format';
import { library } from './library.svelte';
import { attempt, toasts } from './toast.svelte';

/** calibre import: find the library, confirm what will happen, import with progress, report. */
class CalibreStore {
  /** The library calibre uses on this PC, once looked up (null = none found). */
  found = $state<string | null>(null);
  detected = $state(false);
  /** Set while the "Import N books?" dialog is open. */
  pending = $state<CalibreSummary | null>(null);
  inspecting = $state(false);
  running = $state(false);
  progress = $state<CalibreProgress | null>(null);
  /** Outcome of the last import; shown in the floating panel until dismissed. */
  report = $state<CalibreReport | null>(null);

  #detecting = false;

  async detect(force = false) {
    if (this.#detecting || (this.detected && !force)) return;
    this.#detecting = true;
    try {
      const path = await attempt('Could not look for a calibre library', findCalibreLibrary);
      if (path !== undefined) {
        this.found = path;
        this.detected = true;
      }
    } finally {
      this.#detecting = false;
    }
  }

  /** "Choose a different library…": folder picker, then the confirm dialog. */
  async choose() {
    const picked = await attempt('Could not open the folder picker', () => open({ directory: true, multiple: false }));
    if (!picked || Array.isArray(picked)) return;
    await this.prepare(picked);
  }

  /** Looks inside the library, then asks the user to confirm. */
  async prepare(path: string) {
    if (this.inspecting || this.running) return;
    this.inspecting = true;
    try {
      const summary = await attempt('That folder does not look like a calibre library', () => inspectCalibre(path));
      if (summary) this.pending = summary;
    } finally {
      this.inspecting = false;
    }
  }

  cancel() {
    this.pending = null;
  }

  onProgress(p: CalibreProgress) {
    if (this.running) this.progress = p;
  }

  async confirm() {
    const summary = this.pending;
    if (!summary || this.running) return;
    this.pending = null;
    this.running = true;
    this.report = null;
    this.progress = { deviceId: '', phase: 'importing', done: 0, total: summary.bookCount, current: null };
    try {
      const report = await attempt('The calibre import failed', () => importCalibre(summary.path));
      if (report) {
        this.report = report;
        const text = `Imported ${plural(report.imported, 'book')} from calibre.`;
        const extra = report.skippedDuplicates ? ` ${plural(report.skippedDuplicates, 'duplicate')} skipped.` : '';
        if (report.failed.length) {
          toasts.error(`${text}${extra}`, `${plural(report.failed.length, 'book')} could not be imported.`);
        } else {
          toasts.success(text + extra);
        }
      }
    } finally {
      this.running = false;
      this.progress = null;
      void library.refresh();
    }
  }

  dismissReport() {
    this.report = null;
  }
}

export const calibre = new CalibreStore();
