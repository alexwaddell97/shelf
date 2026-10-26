// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
// Keep the shapes here in sync with the serde structs on the Rust side
// (they serialize with camelCase field names).

import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type ReadStatus = 'unread' | 'reading' | 'finished';

export interface Book {
  id: number;
  title: string;
  author: string;
  series: string | null;
  seriesIndex: number | null;
  description: string | null;
  /** Lower-case file extension of the stored file, e.g. "epub". */
  format: string;
  sizeBytes: number;
  /** RFC 3339 timestamp. */
  addedAt: string;
  /** Absolute path to the extracted cover image, if the book has one. */
  coverPath: string | null;
  filePath: string;
  /** 0–100, from the last device sync. */
  progress: number;
  status: ReadStatus;
  /** RFC 3339 timestamp of the last read on a device, if known. */
  lastRead: string | null;
  /** Device ids the book is currently on. */
  onDevices: string[];
  /** Device ids the book is queued to be sent to (or re-sent to, after an edit). */
  queuedFor: string[];
  /** calibre-style sort name: "Herbert, Frank". Use this for sorting by author. */
  authorSort: string | null;
  publisher: string | null;
  /** "YYYY-MM-DD" or "YYYY". */
  published: string | null;
  /** ISBN-10/13 without dashes. */
  isbn: string | null;
  /** Language code as the book gives it, e.g. "en", "eng", "en-GB". */
  language: string | null;
  /** 1–5 stars. */
  rating: number | null;
  /** Descriptive tags (often imported from calibre or the EPUB). */
  tags: string[];
  /** The user's own collections. */
  collections: string[];
}

/**
 * Fields left out are unchanged; `null` clears a field. Multiple authors are joined with
 * " & ". `authorSort: null` (or "") recomputes it from the author. Changing what an
 * e-reader shows (title, author, series, description) or the cover re-queues the book for
 * devices it's already on, so the copy there is updated.
 */
export interface BookPatch {
  title?: string;
  author?: string;
  authorSort?: string | null;
  series?: string | null;
  seriesIndex?: number | null;
  description?: string | null;
  publisher?: string | null;
  published?: string | null;
  isbn?: string | null;
  language?: string | null;
  rating?: number | null;
  tags?: string[];
  collections?: string[];
}

export type LabelKind = 'tag' | 'collection';

export interface Label {
  id: number;
  kind: LabelKind;
  name: string;
  /** Number of books with this label. */
  count: number;
}

export interface CoverCandidate {
  thumbUrl: string;
  url: string;
}

export type SupportLevel = 'full' | 'beta' | 'planned';

export interface Device {
  /** Stable id (e.g. the Kobo serial number). */
  id: string;
  /** Display name, e.g. "Kobo eReader". */
  name: string;
  /** Driver kind, e.g. "kobo". */
  kind: string;
  connected: boolean;
  /** Where the device is mounted while connected, e.g. "E:\\". */
  mountPath: string | null;
  lastSeen: string | null;
  /** When Shelf last finished a sync with this device. */
  lastSynced: string | null;
  supportLevel: SupportLevel;
  /** Format books are sent in, e.g. "kepub" or "epub". */
  sendFormat: string;
  freeBytes: number | null;
  totalBytes: number | null;
}

export interface SyncReport {
  deviceId: string;
  sent: number;
  progressUpdated: number;
  errors: string[];
}

export type SourceKind = 'store' | 'free' | 'custom';

export interface Source {
  id: string;
  name: string;
  kind: SourceKind;
  /** URL with a literal "{query}" placeholder for the search text. */
  urlTemplate: string;
  enabled: boolean;
}

export interface CatalogBook {
  /** Open Library work key, e.g. "/works/OL45883W", or "gutenberg:1342". */
  key: string;
  title: string;
  author: string;
  year: number | null;
  coverUrl: string | null;
  /** True when a public-domain EPUB is available to download directly. */
  isFree: boolean;
  /** Direct EPUB download URL for free books (Project Gutenberg). */
  epubUrl: string | null;
}

export interface WishItem {
  id: number;
  catalogKey: string | null;
  title: string;
  author: string;
  coverUrl: string | null;
  addedAt: string;
  /** Where the wish came from: "search", "goodreads", ... */
  origin: string;
}

export interface Settings {
  libraryDir: string;
  watchFolders: string[];
  autoSend: boolean;
  convertForDevice: boolean;
  backupDeviceDb: boolean;
  setupDone: boolean;
}

export interface Tools {
  calibre: string | null;
}

export type ImportStep = 'reading' | 'cover' | 'saving' | 'done' | 'failed';

export interface ImportProgress {
  file: string;
  step: ImportStep;
  bookId: number | null;
  /** Set when the imported book matched a wishlist item. */
  matchedWishlist: boolean;
  error: string | null;
}

export interface SyncProgress {
  deviceId: string;
  /**
   * checking: started, nothing reported yet (set by the interface)
   * sending: copying books (done of total, current title)
   * reading: books sent; reading the device and updating reading progress
   * importing: a calibre import (see CalibreProgress)
   */
  phase: 'checking' | 'sending' | 'reading' | 'importing';
  done: number;
  total: number;
  current: string | null;
}

export interface CalibreSummary {
  path: string;
  bookCount: number;
  /** Books with no EPUB/KEPUB/AZW3/MOBI/PDF file; they'll be skipped. */
  unsupported: number;
}

export interface CalibreReport {
  imported: number;
  skippedDuplicates: number;
  /** One readable line per book that couldn't be imported. */
  failed: string[];
}

/** Progress of a calibre import (deviceId is always empty). */
export type CalibreProgress = SyncProgress;

export interface GoodreadsAccount {
  userId: string;
  /** Profile owner's first name as Goodreads shows it, if known. */
  name: string | null;
  lastChecked: string | null;
  wantToRead: number;
  reading: number;
  read: number;
  /** Wishlist items added by the last check. */
  lastAdded: number;
}

export interface GoodreadsApplied {
  addedToWishlist: number;
  removedFromWishlist: number;
  markedReading: number;
  markedFinished: number;
}

// --- Library ---
export const listBooks = () => invoke<Book[]>('list_books');
export const getBook = (id: number) => invoke<Book>('get_book', { id });
export const importFiles = (paths: string[]) => invoke<number[]>('import_files', { paths });
export const updateBook = (id: number, patch: BookPatch) => invoke<Book>('update_book', { id, patch });
export const deleteBook = (id: number, deleteFile: boolean) => invoke<void>('delete_book', { id, deleteFile });
export const revealBook = (id: number) => invoke<void>('reveal_book', { id });
export const openBook = (id: number) => invoke<void>('open_book', { id });

// --- Devices ---
export const listDevices = () => invoke<Device[]>('list_devices');
export const refreshDevices = () => invoke<Device[]>('refresh_devices');
export const queueForDevice = (deviceId: string, bookIds: number[]) => invoke<void>('queue_for_device', { deviceId, bookIds });
export const unqueue = (deviceId: string, bookId: number) => invoke<void>('unqueue', { deviceId, bookId });
export const syncDevice = (deviceId: string) => invoke<SyncReport>('sync_device', { deviceId });
export const removeFromDevice = (deviceId: string, bookId: number) => invoke<void>('remove_from_device', { deviceId, bookId });
export const forgetDevice = (deviceId: string) => invoke<void>('forget_device', { deviceId });

// --- Sources, catalogue and wishlist ---
export const listSources = () => invoke<Source[]>('list_sources');
export const saveSources = (sources: Source[]) => invoke<void>('save_sources', { sources });
export const searchCatalog = (query: string) => invoke<CatalogBook[]>('search_catalog', { query });
export const downloadFree = (book: CatalogBook) => invoke<number>('download_free', { book });
export const listWishlist = () => invoke<WishItem[]>('list_wishlist');
export const addToWishlist = (book: CatalogBook, origin = 'search') => invoke<WishItem>('add_to_wishlist', { book, origin });
export const removeFromWishlist = (id: number) => invoke<void>('remove_from_wishlist', { id });
export const openUrl = (url: string) => invoke<void>('open_url', { url });

// --- Settings ---
export const getSettings = () => invoke<Settings>('get_settings');
export const saveSettings = (settings: Settings) => invoke<void>('save_settings', { settings });
export const getTools = () => invoke<Tools>('get_tools');

// --- calibre import (reads a calibre library; never modifies it) ---
/** The library calibre is set up to use on this computer, if any. */
export const findCalibreLibrary = () => invoke<string | null>('find_calibre_library');
export const inspectCalibre = (path: string) => invoke<CalibreSummary>('inspect_calibre', { path });
/** Imports every usable book. Doesn't queue them for devices. Emits calibre-progress. */
export const importCalibre = (path: string) => invoke<CalibreReport>('import_calibre', { path });

// --- Goodreads (one-way, read-only: public shelf feeds or a CSV export) ---
export const getGoodreads = () => invoke<GoodreadsAccount | null>('get_goodreads');
/** `profile` is a profile link (goodreads.com/user/show/12345-name) or the numeric id. */
export const connectGoodreads = (profile: string) => invoke<GoodreadsAccount>('connect_goodreads', { profile });
export const checkGoodreads = () => invoke<GoodreadsAccount>('check_goodreads');
export const disconnectGoodreads = () => invoke<void>('disconnect_goodreads');
/** A Goodreads "Export library" CSV file. */
export const importGoodreadsCsv = (path: string) => invoke<GoodreadsApplied>('import_goodreads_csv', { path });

// --- Tags and collections ---
export const listLabels = (kind: LabelKind) => invoke<Label[]>('list_labels', { kind });
export const createLabel = (kind: LabelKind, name: string) => invoke<void>('create_label', { kind, name });
export const renameLabel = (id: number, name: string) => invoke<void>('rename_label', { id, name });
export const deleteLabel = (id: number) => invoke<void>('delete_label', { id });
/** Adds books to a tag/collection (add = true) or removes them. */
export const labelBooks = (labelId: number, bookIds: number[], add: boolean) =>
  invoke<void>('label_books', { labelId, bookIds, add });

// --- Covers (saved as JPEG; also written into copies sent to e-readers) ---
/** Any JPEG/PNG/WebP/GIF file. */
export const setCoverFromFile = (id: number, path: string) => invoke<Book>('set_cover_from_file', { id, path });
/** Only URLs from findCovers / the catalogue (Open Library, Gutenberg, Goodreads images). */
export const setCoverFromUrl = (id: number, url: string) => invoke<Book>('set_cover_from_url', { id, url });
/** Back to the generated cover. */
export const removeCover = (id: number) => invoke<Book>('remove_cover', { id });
/** Cover options from Open Library for this book's title and author (can take a few seconds). */
export const findCovers = (id: number) => invoke<CoverCandidate[]>('find_covers', { id });

// --- Opening files with Shelf (double-click, Open with) ---
/** A book opened from the file manager before the interface was listening; call once on start. */
export const takePendingOpen = () => invoke<number | null>('take_pending_open');
/** Opens Windows' Default apps settings. Returns false on systems without such a page. */
export const openDefaultAppsSettings = () => invoke<boolean>('open_default_apps_settings');

// --- Reader ---
/** Formats Shelf's own reader opens; others go to another app (openBook). */
export const READABLE_FORMATS = ['epub', 'kepub', 'mobi', 'azw3'];
export interface ReadingPosition {
  /** EPUB CFI from the reader. */
  position: string;
  /** 0–1 through the book. */
  fraction: number;
}
/** Opens the book in its own reader window, or brings that window forward. */
export const openReader = (bookId: number) => invoke<void>('open_reader', { bookId });
/** This computer's Downloads folder, if it has one. */
export const downloadsDir = () => invoke<string | null>('downloads_dir');
/** The book's file as raw bytes, for the reader. */
export const readBookFile = (bookId: number) => invoke<ArrayBuffer>('read_book_file', { bookId });
export const getReadingPosition = (bookId: number) => invoke<ReadingPosition | null>('get_reading_position', { bookId });
/** Also updates the book's progress and status in the library. */
export const saveReadingPosition = (bookId: number, position: string, fraction: number) =>
  invoke<void>('save_reading_position', { bookId, position, fraction });

// --- Events ---
/** An Adobe DRM licence (.acsm) landed in a watched folder; payload is its file name. */
export const onDrmDownload = (cb: (file: string) => void): Promise<UnlistenFn> =>
  listen<string>('drm-download', (e) => cb(e.payload));
/** A file was opened with Shelf: show this book's page. */
export const onOpenBook = (cb: (bookId: number) => void): Promise<UnlistenFn> =>
  listen<number>('open-book', (e) => cb(e.payload));
export const onCalibreProgress = (cb: (p: CalibreProgress) => void): Promise<UnlistenFn> =>
  listen<CalibreProgress>('calibre-progress', (e) => cb(e.payload));
export const onLibraryChanged = (cb: () => void): Promise<UnlistenFn> => listen('library-changed', () => cb());
export const onDevicesChanged = (cb: () => void): Promise<UnlistenFn> => listen('devices-changed', () => cb());
export const onImportProgress = (cb: (p: ImportProgress) => void): Promise<UnlistenFn> =>
  listen<ImportProgress>('import-progress', (e) => cb(e.payload));
export const onSyncProgress = (cb: (p: SyncProgress) => void): Promise<UnlistenFn> =>
  listen<SyncProgress>('sync-progress', (e) => cb(e.payload));

/** Fills a source's URL template with the search text. */
export const sourceUrl = (source: Source, query: string) =>
  source.urlTemplate.replace('{query}', encodeURIComponent(query.trim()));

/** URL the webview can load for a local cover file. */
export const coverSrc = (path: string | null) => (path ? convertFileSrc(path) : null);
