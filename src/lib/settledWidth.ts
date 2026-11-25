// `use:settledWidth={(w) => (width = w)}`: reports an element's width, but only once it has
// stopped changing. Layouts that re-flow every book (the bookcase's rows, the table's
// columns) would otherwise rebuild on every frame of the sidebar sliding or a window resize.

const SETTLE_MS = 120;

export function settledWidth(node: HTMLElement, report: (width: number) => void) {
  let send = report;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let last = node.clientWidth;
  send(last);
  const observer = new ResizeObserver(() => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      const width = node.clientWidth;
      if (width !== last) send((last = width));
    }, SETTLE_MS);
  });
  observer.observe(node);
  return {
    update(next: (width: number) => void) {
      send = next;
    },
    destroy() {
      observer.disconnect();
      clearTimeout(timer);
    }
  };
}
