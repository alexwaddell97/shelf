import type { CatalogBook } from '$lib/api';
import type { Scope } from './library.svelte';

export type Section = 'library' | 'wishlist' | 'devices' | 'sources' | 'accounts';
export type BrowseKind = 'authors' | 'series' | 'collections' | 'tags';

/** Where the back button on a detail page goes. */
export type Back =
  | { kind: 'library' }
  | { kind: 'wishlist' }
  | { kind: 'devices' }
  | { kind: 'search'; query: string }
  | { kind: 'scope'; scope: Scope };

export type View =
  | { name: Exclude<Section, 'library'> }
  | { name: 'library'; scope?: Scope }
  | { name: 'browse'; kind: BrowseKind }
  | { name: 'book'; id: number; back: Back; siblings: number[] }
  | { name: 'catalog'; book: CatalogBook; back: Back };

const SCOPE_BROWSE: Record<Scope['kind'], BrowseKind> = {
  author: 'authors',
  series: 'series',
  collection: 'collections',
  tag: 'tags'
};

export const browseKindOf = (scope: Scope): BrowseKind => SCOPE_BROWSE[scope.kind];

class NavStore {
  view = $state<View>({ name: 'library' });
  /** Header search box. A non-empty value takes over the main area. */
  query = $state('');
  /** The metadata editor is open for the book page being shown. */
  editing = $state(false);

  get searching(): boolean {
    return this.query.trim().length > 0;
  }

  /** Which sidebar entry to highlight. */
  get section(): Section | BrowseKind | 'search' | null {
    if (this.searching) return 'search';
    const v = this.view;
    if (v.name === 'book') {
      if (v.back.kind === 'devices') return 'devices';
      if (v.back.kind === 'scope') return browseKindOf(v.back.scope);
      return 'library';
    }
    if (v.name === 'catalog') return v.back.kind === 'wishlist' ? 'wishlist' : null;
    if (v.name === 'browse') return v.kind;
    if (v.name === 'library' && v.scope) return browseKindOf(v.scope);
    return v.name;
  }

  #leave() {
    this.query = '';
    this.editing = false;
  }

  go(section: Section) {
    this.#leave();
    this.view = { name: section };
  }

  browse(kind: BrowseKind) {
    this.#leave();
    this.view = { name: 'browse', kind };
  }

  showScope(scope: Scope) {
    this.#leave();
    this.view = { name: 'library', scope };
  }

  /** `siblings`: ids of the grid the book was opened from, for Previous/Next in the editor. */
  openBook(id: number, back: Back = { kind: 'library' }, siblings: number[] = []) {
    // Coming from search, remember the query so Back returns to the results.
    const from: Back = this.searching ? { kind: 'search', query: this.query } : back;
    this.#leave();
    this.view = { name: 'book', id, back: from, siblings };
  }

  /** Switches the book page to a neighbouring book without closing the editor. */
  stepBook(id: number) {
    const v = this.view;
    if (v.name === 'book') this.view = { ...v, id };
  }

  openCatalog(book: CatalogBook, back: Back = { kind: 'library' }) {
    const from: Back = this.searching ? { kind: 'search', query: this.query } : back;
    this.#leave();
    this.view = { name: 'catalog', book, back: from };
  }

  goBack(back: Back) {
    switch (back.kind) {
      case 'search':
        this.#leave();
        this.view = { name: 'library' };
        this.query = back.query;
        break;
      case 'scope':
        this.showScope(back.scope);
        break;
      default:
        this.go(back.kind);
    }
  }

  backLabel(back: Back): string {
    switch (back.kind) {
      case 'search':
        return 'Back to results';
      case 'wishlist':
        return 'Wishlist';
      case 'devices':
        return 'Devices';
      case 'scope':
        return back.scope.title;
      default:
        return 'Library';
    }
  }
}

export const nav = new NavStore();

export const seriesScope = (name: string): Scope => ({ kind: 'series', key: name, title: `Series: ${name}` });
export const authorScope = (key: string, name: string): Scope => ({ kind: 'author', key, title: `Author: ${name}` });
export const collectionScope = (name: string): Scope => ({ kind: 'collection', key: name, title: `Collection: ${name}` });
export const tagScope = (name: string): Scope => ({ kind: 'tag', key: name, title: `Tag: ${name}` });
