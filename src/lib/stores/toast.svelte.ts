import { errorMessage } from '$lib/format';

export type ToastKind = 'info' | 'success' | 'error';

export interface Toast {
  id: number;
  kind: ToastKind;
  text: string;
  /** Smaller monospace line under the message (a URL, a path, an error detail). */
  detail?: string;
  /** A button on the toast, e.g. "Open". Clicking it also dismisses the toast. */
  action?: ToastAction;
}

export interface ToastAction {
  label: string;
  run: () => void;
}

class ToastStore {
  items = $state<Toast[]>([]);
  #next = 1;
  #timers = new Map<number, ReturnType<typeof setTimeout>>();

  push(kind: ToastKind, text: string, detail?: string, action?: ToastAction) {
    const id = this.#next++;
    // Keep at most 3 on screen; the oldest make room.
    const kept = this.items.slice(-2);
    for (const t of this.items) if (!kept.includes(t)) this.dismiss(t.id);
    this.items = [...kept, { id, kind, text, detail, action }];
    // Toasts you can act on stay a little longer.
    const ms = kind === 'error' ? 8000 : action ? 9000 : 4000;
    this.#timers.set(
      id,
      setTimeout(() => this.dismiss(id), ms)
    );
    return id;
  }

  info(text: string, detail?: string) {
    return this.push('info', text, detail);
  }
  success(text: string, detail?: string, action?: ToastAction) {
    return this.push('success', text, detail, action);
  }
  error(text: string, detail?: string, action?: ToastAction) {
    return this.push('error', text, detail, action);
  }

  dismiss(id: number) {
    const timer = this.#timers.get(id);
    if (timer) clearTimeout(timer);
    this.#timers.delete(id);
    this.items = this.items.filter((t) => t.id !== id);
  }
}

export const toasts = new ToastStore();

/**
 * Runs a backend call and turns a failure into a readable toast instead of an unhandled
 * rejection. Resolves to the result, or `undefined` if the call failed.
 */
export async function attempt<T>(failureText: string, fn: () => Promise<T>): Promise<T | undefined> {
  try {
    return await fn();
  } catch (e) {
    console.error(failureText, e);
    toasts.error(failureText, errorMessage(e));
    return undefined;
  }
}
