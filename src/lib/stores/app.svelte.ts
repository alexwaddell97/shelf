import { getCurrentWebview } from '@tauri-apps/api/webview';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { onCalibreProgress, onDevicesChanged, onDrmDownload, onImportProgress, onLibraryChanged, onOpenBook, onSyncProgress, takePendingOpen } from '$lib/api';
import { calibre } from './calibre.svelte';
import { devices } from './devices.svelte';
import { goodreads } from './goodreads.svelte';
import { imports } from './imports.svelte';
import { library } from './library.svelte';
import { nav } from './nav.svelte';
import { settings } from './settings.svelte';
import { toasts } from './toast.svelte';
import { wishlist } from './wishlist.svelte';

/**
 * Loads everything the shell needs and subscribes to backend events and window file drops.
 * Returns a cleanup function.
 */
/** Shows a book that was opened from the file manager, loading the library first if it hasn't seen it yet. */
async function showBook(id: number) {
  if (!library.byId(id)) await library.refresh();
  if (library.byId(id)) nav.openBook(id);
  else toasts.error('That book is not in your library yet.');
}

export function startApp(): () => void {
  let stopped = false;
  const unlisteners: UnlistenFn[] = [];

  const subscribe = async (label: string, fn: () => Promise<UnlistenFn>) => {
    try {
      const un = await fn();
      if (stopped) un();
      else unlisteners.push(un);
    } catch (e) {
      // Outside the Tauri shell (plain browser) there is no event bus.
      console.warn(`Could not subscribe to ${label}`, e);
    }
  };

  void settings.load();
  void library.refresh();
  void wishlist.refresh();
  void devices.refresh();
  void goodreads.load();
  void (async () => {
    try {
      const pending = await takePendingOpen();
      if (pending != null && !stopped) await showBook(pending);
    } catch (e) {
      console.warn('Could not check for a book opened from the file manager', e);
    }
  })();

  void subscribe('library-changed', () =>
    onLibraryChanged(() => {
      void library.refresh();
      void wishlist.refresh();
    })
  );
  void subscribe('devices-changed', () =>
    onDevicesChanged(() => {
      void devices.refresh();
      void library.refresh();
    })
  );
  void subscribe('import-progress', () => onImportProgress((p) => imports.onProgress(p)));
  void subscribe('drm-download', () =>
    onDrmDownload((file) =>
      toasts.error(
        `“${file}” is a licence file, not the book`,
        'It’s for Adobe Digital Editions, which swaps it for the DRM-protected book. Books bought on the Kobo Store appear on your Kobo over Wi-Fi by themselves.'
      )
    )
  );
  void subscribe('open-book', () => onOpenBook((id) => void showBook(id)));
  void subscribe('calibre-progress', () => onCalibreProgress((p) => calibre.onProgress(p)));
  void subscribe('sync-progress', () => onSyncProgress((p) => devices.onProgress(p)));

  void subscribe('drag-drop', () =>
    getCurrentWebview().onDragDropEvent((event) => {
      const p = event.payload;
      if (p.type === 'enter') {
        imports.dragging = true;
      } else if (p.type === 'leave') {
        imports.dragging = false;
      } else if (p.type === 'drop') {
        imports.dragging = false;
        if (p.paths.length > 0) void imports.importPaths(p.paths);
      }
    })
  );

  // Don't let a dropped file navigate the webview away when the Tauri drop handler isn't active.
  const swallow = (e: DragEvent) => e.preventDefault();
  window.addEventListener('dragover', swallow);
  window.addEventListener('drop', swallow);

  const onRejection = (e: PromiseRejectionEvent) => {
    console.error('Unhandled rejection', e.reason);
    toasts.error('Something went wrong', e.reason instanceof Error ? e.reason.message : String(e.reason));
  };
  window.addEventListener('unhandledrejection', onRejection);

  return () => {
    stopped = true;
    unlisteners.forEach((un) => un());
    window.removeEventListener('dragover', swallow);
    window.removeEventListener('drop', swallow);
    window.removeEventListener('unhandledrejection', onRejection);
  };
}
