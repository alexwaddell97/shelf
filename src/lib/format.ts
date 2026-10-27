import type { Book, CatalogBook, WishItem } from './api';

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return '';
  let n = bytes;
  let i = 0;
  while (n >= 1000 && i < UNITS.length - 1) {
    n /= 1000;
    i++;
  }
  const digits = i === 0 ? 0 : 1;
  return `${n.toFixed(digits)} ${UNITS[i]}`;
}

const dateFmt = new Intl.DateTimeFormat(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
const shortFmt = new Intl.DateTimeFormat(undefined, { day: 'numeric', month: 'short' });

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return '';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? '' : dateFmt.format(d);
}

/** "Today", "Yesterday", "3 Sep" (same year) or "3 Sep 2025". */
export function relativeDate(iso: string | null | undefined): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  const now = new Date();
  const startOf = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOf(now) - startOf(d)) / 86_400_000);
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  return d.getFullYear() === now.getFullYear() ? shortFmt.format(d) : dateFmt.format(d);
}

export function plural(n: number, one: string, many = one + 's'): string {
  return `${n} ${n === 1 ? one : many}`;
}

export function statusLabel(book: Pick<Book, 'status' | 'progress'>): string {
  if (book.status === 'finished') return 'Finished';
  if (book.status === 'reading') return `${Math.round(book.progress)}% read`;
  return 'Not started';
}

export function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function extension(path: string): string {
  const name = basename(path);
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : '';
}

export function clamp(n: number, lo = 0, hi = 100): number {
  return Math.min(hi, Math.max(lo, n));
}

/** Lower-cased "title author" key for loose matching of catalogue items. */
export function looseKey(title: string, author: string): string {
  const norm = (s: string) => s.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, ' ').trim();
  return `${norm(title)}|${norm(author)}`;
}

export function wishToCatalog(w: WishItem): CatalogBook {
  return {
    key: w.catalogKey ?? '',
    title: w.title,
    author: w.author,
    year: null,
    coverUrl: w.coverUrl,
    isFree: false,
    epubUrl: null
  };
}

/** Human-friendly message for anything thrown by an invoke call. */
export function errorMessage(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === 'object') {
    const m = (e as { message?: unknown }).message;
    if (typeof m === 'string') return m;
    try {
      return JSON.stringify(e);
    } catch {
      /* fall through */
    }
  }
  return 'Something went wrong.';
}

/** "just now", "5 minutes ago", "3 hours ago", then a date. */
export function timeAgo(iso: string | null | undefined): string {
  if (!iso) return '';
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return '';
  const mins = Math.round((Date.now() - t) / 60_000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins} minute${mins === 1 ? '' : 's'} ago`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`;
  return relativeDate(iso);
}

/** Title for sorting: lower-case, without a leading "The", "A" or "An". */
export function sortTitle(title: string): string {
  return title.trim().replace(/^(the|a|an)\s+/i, '').toLowerCase();
}

/** Series position the way calibre shows it: 1, 2, 1.5. */
export function formatIndex(n: number): string {
  return String(Number(n.toFixed(2)));
}

/** "The Expanse · 1", or just the series name when there is no number. */
export function seriesLabel(b: Pick<Book, 'series' | 'seriesIndex'>): string {
  if (!b.series) return '';
  return b.seriesIndex != null ? `${b.series} · ${formatIndex(b.seriesIndex)}` : b.series;
}

/**
 * The individual authors of a book, each with the key used to sort and group them.
 * Books by "A & B" appear under both authors. Sort keys come from `authorSort` when it
 * splits the same way, otherwise the display name is the key.
 */
export function authorEntries(b: Pick<Book, 'author' | 'authorSort'>): { key: string; name: string }[] {
  const names = b.author
    .split(/\s+&\s+/)
    .map((s) => s.trim())
    .filter(Boolean);
  if (names.length === 0) return [{ key: 'Unknown author', name: 'Unknown author' }];
  const sorts = (b.authorSort ?? '')
    .split(/\s+&\s+/)
    .map((s) => s.trim())
    .filter(Boolean);
  const entries = names.map((name, i) => ({ name, key: sorts.length === names.length ? sorts[i] : name }));
  // Some files list one person twice ("Cixin Liu & Liu, Cixin"); keep the first. Keys
  // must be unique too, since lists of authors are keyed by them.
  const seen = new Set<string>();
  return entries.filter((e) => {
    const ids = [personKey(e.name), e.key.toLowerCase()];
    if (ids.some((id) => seen.has(id))) return false;
    ids.forEach((id) => seen.add(id));
    return true;
  });
}

/** The same for "Cixin Liu" and "Liu, Cixin": the name's words, lower-cased and sorted. */
function personKey(name: string): string {
  return name
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean)
    .sort()
    .join(' ');
}

// Three-letter codes some files use, mapped to the two-letter ones Intl knows.
const LANGUAGE_CODES: Record<string, string> = {
  eng: 'en', fra: 'fr', fre: 'fr', deu: 'de', ger: 'de', nld: 'nl', dut: 'nl', spa: 'es', ita: 'it',
  por: 'pt', rus: 'ru', jpn: 'ja', zho: 'zh', chi: 'zh', swe: 'sv', nor: 'no', dan: 'da', fin: 'fi', pol: 'pl'
};

/** "eng" or "en-US" → "English"; null for "und" (undetermined) or nothing. */
export function languageName(code: string | null | undefined): string | null {
  const raw = code?.trim();
  if (!raw || raw.toLowerCase() === 'und') return null;
  const tag = LANGUAGE_CODES[raw.toLowerCase()] ?? raw;
  try {
    return new Intl.DisplayNames(['en'], { type: 'language' }).of(tag) ?? raw;
  } catch {
    return raw;
  }
}

/** Sort key for an author: the calibre sort name, or the name itself. */
export function authorSortKey(b: Pick<Book, 'author' | 'authorSort'>): string {
  return (b.authorSort?.trim() || b.author).toLowerCase();
}
