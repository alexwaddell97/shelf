import { getVersion } from '@tauri-apps/api/app';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { errorMessage } from '$lib/format';
import { devices } from './devices.svelte';
import { toasts } from './toast.svelte';

// New versions of Shelf. The app checks a small file on the latest GitHub release a little
// after it starts and then every few hours; when there's something newer, a popup offers
// it. Dismissing it hides that version's popup for good; it stays in Settings (with a dot on
// the Settings item) until installed. Updates are signed, and the updater refuses any
// download that doesn't match Shelf's public key.

const FIRST_CHECK_MS = 15_000;
const EVERY_MS = 6 * 60 * 60 * 1000;
const DISMISSED_KEY = 'shelf.updateDismissed';

export type UpdatePhase = 'idle' | 'checking' | 'downloading' | 'installing';

class UpdateStore {
  /** This copy's version, e.g. "0.1.0". */
  current = $state('');
  /** A newer version that's ready to install. */
  available = $state<{ version: string; notes: string } | null>(null);
  phase = $state<UpdatePhase>('idle');
  /** 0–1 while downloading, when the size is known. */
  progress = $state<number | null>(null);
  lastChecked = $state<Date | null>(null);
  /** The version whose popup was dismissed. */
  dismissed = $state(loadDismissed());

  #update: Update | null = null;

  get busy() {
    return this.phase === 'downloading' || this.phase === 'installing';
  }

  /** Show the popup: a new version you haven't dismissed, or one that's installing. */
  get prompting() {
    return !!this.available && (this.available.version !== this.dismissed || this.busy);
  }

  /** "Not now": hide this version's popup. Settings still offers it. */
  dismiss() {
    if (!this.available) return;
    this.dismissed = this.available.version;
    try {
      localStorage.setItem(DISMISSED_KEY, this.dismissed);
    } catch {
      /* storage unavailable */
    }
  }

  /** Starts background checks. Returns a function that stops them. */
  start(): () => void {
    void getVersion().then((v) => (this.current = v), () => {});
    // A development build has no releases to compare against.
    if (import.meta.env.DEV) return () => {};
    const first = setTimeout(() => void this.check(), FIRST_CHECK_MS);
    const every = setInterval(() => void this.check(), EVERY_MS);
    return () => {
      clearTimeout(first);
      clearInterval(every);
    };
  }

  /** Looks for a newer version. `manual` reports the result, including "you're up to date". */
  async check(manual = false) {
    if (this.phase !== 'idle') return;
    this.phase = 'checking';
    try {
      const update = await check({ timeout: 20_000 });
      this.lastChecked = new Date();
      if (update) {
        this.#update = update;
        this.available = { version: update.version, notes: update.body?.trim() ?? '' };
      } else {
        this.#update = null;
        this.available = null;
        if (manual) toasts.success('Shelf is up to date', `You have the latest version, ${this.current}.`);
      }
    } catch (e) {
      // Offline, or no release published yet: stay quiet unless asked.
      if (manual) toasts.error('Could not check for updates', errorMessage(e));
      else console.warn('Update check failed', e);
    } finally {
      this.phase = 'idle';
    }
  }

  /** Downloads and installs the update, then restarts Shelf. */
  async install() {
    const update = this.#update;
    if (!update || this.phase !== 'idle') return;
    if (devices.syncing) {
      toasts.info('Finish syncing first', 'Shelf restarts to update, so wait until your e-reader is done.');
      return;
    }
    this.phase = 'downloading';
    this.progress = null;
    let total = 0;
    let done = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') total = event.data.contentLength ?? 0;
        else if (event.event === 'Progress') {
          done += event.data.chunkLength;
          this.progress = total ? Math.min(1, done / total) : null;
        } else if (event.event === 'Finished') this.phase = 'installing';
      });
      // On Windows the installer has already closed Shelf by now; elsewhere, restart into
      // the new version.
      await relaunch();
    } catch (e) {
      this.phase = 'idle';
      this.progress = null;
      toasts.error('The update did not install', errorMessage(e));
    }
  }
}

function loadDismissed(): string | null {
  try {
    return localStorage.getItem(DISMISSED_KEY);
  } catch {
    return null;
  }
}

export const updates = new UpdateStore();
