// A small styled hover card: `<span use:tip={'Heading\nDetail line'}>`. The first line is
// bold; the rest is muted detail. It appears quicker than the browser's own tooltip, sits
// above the element (below when there's no room), and stays inside the window.

type TipText = string | null | undefined;

let bubble: HTMLDivElement | null = null;
let showTimer: ReturnType<typeof setTimeout> | undefined;
let owner: HTMLElement | null = null;

function getBubble(): HTMLDivElement {
  if (!bubble) {
    bubble = document.createElement('div');
    bubble.className = 'shelf-tip';
    bubble.setAttribute('role', 'tooltip');
    document.body.appendChild(bubble);
  }
  return bubble;
}

function render(el: HTMLDivElement, text: string) {
  const [head, ...rest] = text.split('\n');
  el.replaceChildren();
  const strong = document.createElement('strong');
  strong.textContent = head;
  el.appendChild(strong);
  for (const line of rest.filter(Boolean)) {
    const span = document.createElement('span');
    span.textContent = line;
    el.appendChild(span);
  }
}

function place(el: HTMLDivElement, target: HTMLElement) {
  const r = target.getBoundingClientRect();
  const b = el.getBoundingClientRect();
  const gap = 8;
  const margin = 8;
  let top = r.top - b.height - gap;
  if (top < margin) top = r.bottom + gap;
  let left = r.left + r.width / 2 - b.width / 2;
  left = Math.max(margin, Math.min(left, window.innerWidth - b.width - margin));
  el.style.top = `${Math.round(top)}px`;
  el.style.left = `${Math.round(left)}px`;
}

function hide() {
  clearTimeout(showTimer);
  owner = null;
  bubble?.classList.remove('shown');
}

export function tip(node: HTMLElement, text: TipText) {
  let current = text;

  function show(delay: number) {
    clearTimeout(showTimer);
    if (!current) return;
    showTimer = setTimeout(() => {
      if (!current) return;
      const el = getBubble();
      render(el, current);
      el.classList.add('shown');
      place(el, node);
      owner = node;
    }, delay);
  }

  const enter = () => show(250);
  const focus = () => {
    if (node.matches(':focus-visible')) show(0);
  };

  node.addEventListener('pointerenter', enter);
  node.addEventListener('pointerleave', hide);
  node.addEventListener('pointerdown', hide);
  node.addEventListener('focus', focus);
  node.addEventListener('blur', hide);
  window.addEventListener('scroll', hide, true);

  return {
    update(next: TipText) {
      current = next;
      // Keep a visible card in step with live state (e.g. sync progress).
      if (owner === node && bubble) {
        if (next) {
          render(bubble, next);
          place(bubble, node);
        } else hide();
      }
    },
    destroy() {
      if (owner === node) hide();
      node.removeEventListener('pointerenter', enter);
      node.removeEventListener('pointerleave', hide);
      node.removeEventListener('pointerdown', hide);
      node.removeEventListener('focus', focus);
      node.removeEventListener('blur', hide);
      window.removeEventListener('scroll', hide, true);
    }
  };
}
