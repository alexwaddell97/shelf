import { getCurrentWindow } from '@tauri-apps/api/window';

export type ThemePref = 'system' | 'light' | 'dark';

const KEY = 'shelf.theme';

function load(): ThemePref {
  try {
    const v = localStorage.getItem(KEY);
    if (v === 'light' || v === 'dark' || v === 'system') return v;
  } catch {
    /* storage unavailable */
  }
  return 'light';
}

/**
 * Light / dark / follow-the-system. `data-theme` on <html> always holds the resolved theme
 * ("light" or "dark"); the inline script in app.html sets it before the first paint and this
 * store keeps it right afterwards, including when the OS theme flips under "System".
 */
class ThemeStore {
  pref = $state<ThemePref>(load());
  systemDark = $state(true);

  #media: MediaQueryList | undefined;

  get resolved(): 'light' | 'dark' {
    return this.pref === 'system' ? (this.systemDark ? 'dark' : 'light') : this.pref;
  }

  /** Call once on start. Returns a cleanup function. */
  init(): () => void {
    this.#media = window.matchMedia('(prefers-color-scheme: dark)');
    this.systemDark = this.#media.matches;
    const onChange = (e: MediaQueryListEvent) => {
      this.systemDark = e.matches;
      this.#apply();
    };
    this.#media.addEventListener('change', onChange);
    this.#apply();
    return () => this.#media?.removeEventListener('change', onChange);
  }

  set(pref: ThemePref) {
    this.pref = pref;
    try {
      localStorage.setItem(KEY, pref);
    } catch {
      /* storage unavailable */
    }
    this.#apply();
  }

  /** The sun/moon button: flips between light and dark. */
  toggle() {
    this.set(this.resolved === 'dark' ? 'light' : 'dark');
  }

  #apply() {
    document.documentElement.dataset.theme = this.resolved;
    void this.#syncWindow();
  }

  /** Tells the native window, so its frame and the area behind the page match. */
  async #syncWindow() {
    try {
      const win = getCurrentWindow();
      await win.setTheme(this.pref === 'system' ? null : this.pref);
      // Read the ground colour from the stylesheet so it can't drift from the tokens.
      const raw = getComputedStyle(document.documentElement).getPropertyValue('--bg').trim();
      const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(raw);
      if (m) await win.setBackgroundColor([parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16), 255]);
    } catch {
      // Not running inside Tauri (or the permission is missing): the page theme still works.
    }
  }
}

export const theme = new ThemeStore();
