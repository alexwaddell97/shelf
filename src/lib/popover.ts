// Closes a popover on Escape or a click outside it: `<div use:popover={() => (open = false)}>`.
// Clicks on the popover's parent (usually the button that opened it) don't count as outside,
// so that button can toggle it. Escape works wherever focus is, not just inside the popover.

export function popover(node: HTMLElement, close: () => void) {
  let done = close;
  const onDown = (e: MouseEvent) => {
    const t = e.target as Node;
    if (!node.contains(t) && !node.parentElement?.contains(t)) done();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.stopPropagation();
      done();
    }
  };
  document.addEventListener('mousedown', onDown);
  // Capture, so the popover closes before anything else (a dialog, the page) handles Escape.
  document.addEventListener('keydown', onKey, true);
  return {
    update(next: () => void) {
      done = next;
    },
    destroy() {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey, true);
    }
  };
}
