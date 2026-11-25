// Window-level layout: whether the sidebar shows just icons. You can collapse it yourself
// (remembered), and narrow windows always get the icon rail.

const KEY = 'shelf.sidebar';
const NARROW = '(max-width: 900px)';

class UiStore {
  /** Collapsed by choice. */
  collapsed = $state(load());
  /** Too narrow for the full sidebar, whatever the choice. */
  narrow = $state(typeof window !== 'undefined' && window.matchMedia(NARROW).matches);

  /** The sidebar is showing icons only. */
  get rail() {
    return this.collapsed || this.narrow;
  }

  constructor() {
    if (typeof window === 'undefined') return;
    window.matchMedia(NARROW).addEventListener('change', (e) => (this.narrow = e.matches));
  }

  toggleSidebar() {
    this.collapsed = !this.collapsed;
    try {
      localStorage.setItem(KEY, this.collapsed ? 'icons' : 'full');
    } catch {
      /* storage unavailable */
    }
  }
}

function load(): boolean {
  try {
    return localStorage.getItem(KEY) === 'icons';
  } catch {
    return false;
  }
}

export const ui = new UiStore();
