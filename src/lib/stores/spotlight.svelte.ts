// Whether the Spotlight search window is open. Ctrl+K / Ctrl+Space (Cmd on a Mac) opens it.

class SpotlightStore {
  open = $state(false);

  show() {
    this.open = true;
  }

  hide() {
    this.open = false;
  }

  toggle() {
    this.open = !this.open;
  }
}

export const spotlight = new SpotlightStore();
