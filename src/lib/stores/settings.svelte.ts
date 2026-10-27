import { getSettings, getTools, listSources, saveSettings, saveSources, type Settings, type Source, type Tools } from '$lib/api';
import { open } from '@tauri-apps/plugin-dialog';
import { attempt, toasts } from './toast.svelte';

class SettingsStore {
  value = $state<Settings | null>(null);
  tools = $state<Tools | null>(null);
  sources = $state<Source[]>([]);
  sourcesLoaded = $state(false);
  /** Set by "Run first-time setup again"; cleared when the wizard closes. */
  rerunSetup = $state(false);

  /** The wizard shows on a fresh install, or when the user asks for it again. */
  get showSetup(): boolean {
    return this.value !== null && (!this.value.setupDone || this.rerunSetup);
  }

  async load() {
    const [settings, tools, sources] = await Promise.all([
      attempt('Could not load your settings', getSettings),
      attempt('Could not check for helper tools', getTools),
      attempt('Could not load your sources', listSources)
    ]);
    if (settings) this.value = settings;
    if (tools) this.tools = tools;
    if (sources) {
      this.sources = sources;
      this.sourcesLoaded = true;
    }
  }

  /** Optimistically applies a partial change and rolls back if the backend refuses it. */
  async save(patch: Partial<Settings>): Promise<boolean> {
    const current = this.value;
    if (!current) return false;
    const prev = Object.fromEntries(Object.keys(patch).map((k) => [k, current[k as keyof Settings]]));
    this.value = { ...current, ...patch };
    // Each save writes the latest settings, in order, so quick changes can't overtake each other.
    const ok = await this.#inOrder(() =>
      attempt('Could not save your settings', async () => {
        await saveSettings($state.snapshot(this.value) as Settings);
        return true;
      })
    );
    if (!ok) {
      if (this.value) this.value = { ...this.value, ...prev };
      return false;
    }
    return true;
  }

  async saveSources(next: Source[]): Promise<boolean> {
    const prev = this.sources;
    this.sources = next;
    const ok = await this.#inOrder(() =>
      attempt('Could not save your sources', async () => {
        await saveSources($state.snapshot(this.sources) as Source[]);
        return true;
      })
    );
    if (!ok) {
      // Roll back only if nothing newer has replaced this change.
      if (this.sources === next) this.sources = prev;
      return false;
    }
    return true;
  }

  #chain: Promise<unknown> = Promise.resolve();

  #inOrder<T>(job: () => Promise<T>): Promise<T> {
    const run = this.#chain.then(job, job);
    this.#chain = run.catch(() => undefined);
    return run;
  }

  enabledSources(kind: Source['kind']): Source[] {
    return this.sources.filter((s) => s.enabled && s.kind === kind);
  }

  /** Folder picker, then adds the choice to the watch folders. */
  async addWatchFolder(): Promise<void> {
    const current = this.value;
    if (!current) return;
    const picked = await attempt('Could not open the folder picker', () => open({ directory: true, multiple: false }));
    if (!picked || Array.isArray(picked)) return;
    if (current.watchFolders.includes(picked)) {
      toasts.info('Shelf is already watching that folder.');
      return;
    }
    await this.save({ watchFolders: [...current.watchFolders, picked] });
  }

  async removeWatchFolder(path: string): Promise<void> {
    const current = this.value;
    if (!current) return;
    await this.save({ watchFolders: current.watchFolders.filter((f) => f !== path) });
  }

  async finishSetup() {
    this.rerunSetup = false;
    if (this.value && !this.value.setupDone) await this.save({ setupDone: true });
  }
}

export const settings = new SettingsStore();
