import type { Action } from 'svelte/action';

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Keeps keyboard focus inside a modal, focuses `[data-autofocus]` (or the first control)
 * on mount, and restores focus to whatever had it before when the modal goes away.
 * Optional parameter: a function called on Escape.
 */
export const trapFocus: Action<HTMLElement, (() => void) | undefined> = (node, onEscape) => {
  const previous = document.activeElement as HTMLElement | null;
  let escape = onEscape;

  const items = () => Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => el.offsetParent !== null);

  queueMicrotask(() => {
    const target = node.querySelector<HTMLElement>('[data-autofocus]') ?? items()[0] ?? node;
    target.focus();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && escape) {
      e.stopPropagation();
      escape();
      return;
    }
    if (e.key !== 'Tab') return;
    const list = items();
    if (list.length === 0) {
      e.preventDefault();
      return;
    }
    const first = list[0];
    const last = list[list.length - 1];
    const active = document.activeElement;
    if (e.shiftKey && (active === first || !node.contains(active))) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (active === last || !node.contains(active))) {
      e.preventDefault();
      first.focus();
    }
  }

  node.addEventListener('keydown', onKey);
  return {
    update(next) {
      escape = next;
    },
    destroy() {
      node.removeEventListener('keydown', onKey);
      if (previous && document.contains(previous)) previous.focus();
    }
  };
};
