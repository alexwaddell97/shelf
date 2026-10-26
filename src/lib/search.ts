// Library search for the header's suggestions: matching that ignores case and accents
// ("bronte" finds "Brontë"), ranking, and splitting text into highlighted pieces.

import type { Book } from '$lib/api';

export interface Segment {
  text: string;
  hit: boolean;
}

/** Lower-cases and strips accents, keeping a map from each folded character back to the original. */
function foldWithMap(text: string): { folded: string; map: number[] } {
  let folded = '';
  const map: number[] = [];
  for (let i = 0; i < text.length; ) {
    const ch = String.fromCodePoint(text.codePointAt(i)!);
    const f = ch.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
    folded += f;
    for (let k = 0; k < f.length; k++) map.push(i);
    i += ch.length;
  }
  map.push(text.length);
  return { folded, map };
}

export const fold = (text: string) => foldWithMap(text).folded;

/** The words of a query, folded. */
export const tokenize = (query: string) => fold(query).split(/\s+/).filter(Boolean);

/** Splits `text` into pieces, marking every place a query word appears. */
export function highlight(text: string, words: string[]): Segment[] {
  if (!text) return [];
  const { folded, map } = foldWithMap(text);
  const marked = new Array<boolean>(text.length).fill(false);
  for (const word of words) {
    for (let at = folded.indexOf(word); at !== -1; at = folded.indexOf(word, at + word.length)) {
      const start = map[at];
      const end = map[at + word.length] ?? text.length;
      for (let i = start; i < end; i++) marked[i] = true;
    }
  }
  const segments: Segment[] = [];
  for (let i = 0; i < text.length; i++) {
    const last = segments[segments.length - 1];
    if (last && last.hit === marked[i]) last.text += text[i];
    else segments.push({ text: text[i], hit: marked[i] });
  }
  return segments;
}

/** How well `word` matches `field`: 3 at the start, 2 at the start of a word, 1 anywhere. */
function wordScore(field: string, word: string): number {
  if (!field) return 0;
  if (field.startsWith(word)) return 3;
  if (field.includes(' ' + word) || field.includes('-' + word)) return 2;
  return field.includes(word) ? 1 : 0;
}

/**
 * Scores a book against the query words, or returns 0 if any word appears nowhere.
 * Title matches count most, then author, then series, then tags.
 */
export function scoreBook(book: Book, words: string[]): number {
  if (!words.length) return 0;
  const title = fold(book.title);
  const author = fold(book.author);
  const series = fold(book.series ?? '');
  const tags = fold(book.tags.join(' '));
  let total = 0;
  for (const word of words) {
    const s = Math.max(wordScore(title, word) * 4, wordScore(author, word) * 3, wordScore(series, word) * 2, wordScore(tags, word));
    if (s === 0) return 0;
    total += s;
  }
  // Prefer titles that start with the whole query.
  if (title.startsWith(words.join(' '))) total += 6;
  return total;
}

/** Best-matching books, highest score first. */
export function searchBooks(books: Book[], query: string, limit = 8): Book[] {
  const words = tokenize(query);
  if (!words.length) return [];
  return books
    .map((book) => ({ book, score: scoreBook(book, words) }))
    .filter((r) => r.score > 0)
    .sort((a, b) => b.score - a.score || a.book.title.localeCompare(b.book.title))
    .slice(0, limit)
    .map((r) => r.book);
}

/** Distinct values (authors, series) where every query word appears, best first. */
export function searchNames(names: string[], query: string, limit = 3): string[] {
  const words = tokenize(query);
  if (!words.length) return [];
  return [...new Set(names)]
    .map((name) => {
      const f = fold(name);
      const parts = words.map((w) => wordScore(f, w));
      return { name, score: parts.includes(0) ? 0 : parts.reduce((a, b) => a + b, 0) };
    })
    .filter((r) => r.score > 0)
    .sort((a, b) => b.score - a.score || a.name.localeCompare(b.name))
    .slice(0, limit)
    .map((r) => r.name);
}

// --- Is this the same book? (mirrors `same_work` in src-tauri/src/db.rs) ---

const letters = (text: string) => fold(text.replace(/&/g, 'and')).replace(/[^\p{L}\p{N}]/gu, '');

/** Drops a leading "The", "A" or "An", which catalogues often disagree on. */
const withoutArticle = (title: string) => title.trim().replace(/^(the|an|a)\s+/i, '');

/** The title without its subtitle: "The Hobbit, or There and Back Again" → "hobbit". */
function mainTitle(title: string): string {
  const lower = title.toLowerCase();
  const cuts = [':', '(', ';', ', or ', ' - '].map((sep) => lower.indexOf(sep)).filter((i) => i >= 0);
  return letters(withoutArticle(cuts.length ? title.slice(0, Math.min(...cuts)) : title));
}

function lastName(author: string): string {
  const first = author.split(/[&,;]/)[0] ?? author;
  const words = first.trim().split(/\s+/);
  return letters(words[words.length - 1] ?? '');
}

/** A title's words, folded, without punctuation or anything from the first "(" on. */
function titleWords(title: string): string[] {
  const cut = title.indexOf('(');
  return fold((cut >= 0 ? title.slice(0, cut) : title).replace(/&/g, ' and ').replace(/['’]/g, ''))
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean);
}

/**
 * True when one title is the end of the other, as when a franchise or series name comes
 * first: "Star Wars Inquisitor: Rise of the Red Blade" and "Inquisitor: Rise of the Red
 * Blade". The shorter title needs three words or more (counting a leading "The").
 */
function endsWithTitle(a: string, b: string): boolean {
  const [wa, wb] = [titleWords(a), titleWords(b)];
  const [short, long] = wa.length <= wb.length ? [wa, wb] : [wb, wa];
  if (short.length < 3) return false;
  const tail = ['the', 'a', 'an'].includes(short[0]) ? short.slice(1) : short;
  if (tail.length >= long.length) return false;
  const offset = long.length - tail.length;
  return tail.every((w, i) => w === long[offset + i]);
}

/** True when two title/author pairs are probably the same work (subtitles and accents ignored). */
export function sameWork(a: { title: string; author: string }, b: { title: string; author: string }): boolean {
  const ta = letters(withoutArticle(a.title));
  const tb = letters(withoutArticle(b.title));
  if (!ta || !tb) return false;
  const titles = ta === tb || mainTitle(a.title) === mainTitle(b.title) || endsWithTitle(a.title, b.title);
  const la = lastName(a.author);
  const lb = lastName(b.author);
  return titles && (!la || !lb || la === lb);
}
