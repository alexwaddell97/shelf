<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    findCovers,
    removeCover,
    setCoverFromFile,
    setCoverFromUrl,
    updateBook,
    type Book,
    type BookPatch,
    type CoverCandidate
  } from '$lib/api';
  import { errorMessage } from '$lib/format';
  import { devices } from '$lib/stores/devices.svelte';
  import { labels } from '$lib/stores/labels.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { nav } from '$lib/stores/nav.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';
  import ChipInput from './ChipInput.svelte';
  import Cover from './Cover.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';
  import Stars from './Stars.svelte';

  let { id }: { id: number } = $props();

  interface Form {
    title: string;
    author: string;
    authorSort: string;
    /** "auto" was clicked: recompute the author sort on save. */
    authorSortAuto: boolean;
    series: string;
    seriesIndex: number | null;
    rating: number | null;
    collections: string[];
    tags: string[];
    publisher: string;
    published: string;
    language: string;
    isbn: string;
    description: string;
  }

  const fromBook = (b: Book): Form => ({
    title: b.title,
    author: b.author,
    authorSort: b.authorSort ?? '',
    authorSortAuto: false,
    series: b.series ?? '',
    seriesIndex: b.seriesIndex,
    rating: b.rating,
    collections: [...b.collections],
    tags: [...b.tags],
    publisher: b.publisher ?? '',
    published: b.published ?? '',
    language: b.language ?? '',
    isbn: b.isbn ?? '',
    description: b.description ?? ''
  });

  const book = $derived(library.byId(id));
  let form = $state<Form | null>(null);
  let loadedId: number | null = null;
  let problem = $state<string | null>(null);
  let saving = $state(false);

  // Load the form whenever the editor is pointed at a different book.
  $effect(() => {
    const b = book;
    if (b && b.id !== loadedId) {
      loadedId = b.id;
      form = fromBook(b);
      problem = null;
      covers = { status: 'idle', items: [], error: null };
    }
  });

  // ---- Previous / Next ----
  const siblings = $derived.by(() => {
    const v = nav.view;
    const own = v.name === 'book' && v.siblings.length > 0 ? v.siblings.filter((s) => library.byId(s)) : [];
    return own.length > 0 ? own : library.list().map((b) => b.id);
  });
  const position = $derived(siblings.indexOf(id));
  const prevId = $derived(position > 0 ? siblings[position - 1] : null);
  const nextId = $derived(position >= 0 && position < siblings.length - 1 ? siblings[position + 1] : null);

  // ---- Validation and the patch ----
  const norm = (s: string) => s.trim() || null;
  const sameSet = (a: string[], b: string[]) => a.length === b.length && [...a].sort().join('\n') === [...b].sort().join('\n');

  function validPublished(v: string): boolean {
    const m = /^(\d{4})(?:-(\d{2})-(\d{2}))?$/.exec(v);
    if (!m) return false;
    if (!m[2]) return true;
    const d = new Date(`${v}T00:00:00Z`);
    return !Number.isNaN(d.getTime()) && d.toISOString().slice(0, 10) === v;
  }

  function validIsbn(v: string): boolean {
    return /^(\d{9}[\dX]|\d{13})$/.test(v);
  }

  function build(b: Book, f: Form): { patch: BookPatch } | { error: string } {
    const patch: BookPatch = {};
    const title = f.title.trim();
    if (!title) return { error: 'A book needs a title.' };
    if (title !== b.title) patch.title = title;

    const author = f.author
      .split('&')
      .map((s) => s.trim())
      .filter(Boolean)
      .join(' & ');
    if (author !== b.author) patch.author = author;

    const sortText = f.authorSort.trim();
    if (f.authorSortAuto) patch.authorSort = null;
    else if (sortText !== (b.authorSort ?? '')) patch.authorSort = sortText || null;
    else if (patch.author !== undefined) patch.authorSort = null; // author changed, sort name untouched: recompute

    const series = norm(f.series);
    if (series !== b.series) patch.series = series;
    let index: number | null = null;
    if (series && f.seriesIndex != null) {
      if (!Number.isFinite(f.seriesIndex) || f.seriesIndex < 0) return { error: 'Series number should be 0 or more, like 1 or 2.5.' };
      index = f.seriesIndex;
    }
    if (index !== b.seriesIndex) patch.seriesIndex = index;

    if (f.rating !== b.rating) patch.rating = f.rating;
    if (!sameSet(f.collections, b.collections)) patch.collections = f.collections;
    if (!sameSet(f.tags, b.tags)) patch.tags = f.tags;

    const publisher = norm(f.publisher);
    if (publisher !== b.publisher) patch.publisher = publisher;

    const published = norm(f.published);
    if (published && !validPublished(published)) return { error: 'Published should be a year (2015) or a date (2015-06-30).' };
    if (published !== b.published) patch.published = published;

    const language = norm(f.language);
    if (language !== b.language) patch.language = language;

    const isbn = f.isbn.replace(/[\s-]/g, '').toUpperCase() || null;
    if (isbn && !validIsbn(isbn)) return { error: 'An ISBN has 10 or 13 characters (dashes and spaces are fine).' };
    if (isbn !== b.isbn) patch.isbn = isbn;

    const description = norm(f.description);
    if (description !== norm(b.description ?? '')) patch.description = description;

    return { patch };
  }

  const dirty = $derived.by(() => {
    if (!book || !form) return false;
    const r = build(book, form);
    return 'error' in r || Object.keys(r.patch).length > 0;
  });

  function deviceNote(b: Book): string | null {
    if (b.onDevices.length === 0) return null;
    const names = b.onDevices.map((d) => devices.name(d));
    return names.length === 1
      ? `Your ${names[0]}’s copy will update on the next sync.`
      : 'The copies on your e-readers will update on the next sync.';
  }

  /** Saves the changed fields in one call. Resolves to false if something needs fixing first. */
  async function save(): Promise<boolean> {
    if (!book || !form || saving) return false;
    const r = build(book, form);
    if ('error' in r) {
      problem = r.error;
      return false;
    }
    problem = null;
    if (Object.keys(r.patch).length === 0) return true;
    saving = true;
    try {
      const updated = await attempt('Could not save the details', () => updateBook(book.id, r.patch));
      if (!updated) return false;
      library.upsert(updated);
      form = fromBook(updated);
      void labels.refresh();
      const note = deviceNote(updated);
      toasts.success(note ? `Details saved. ${note}` : 'Details saved.');
      return true;
    } finally {
      saving = false;
    }
  }

  const close = () => (nav.editing = false);

  async function saveAndClose() {
    if (await save()) close();
  }

  async function step(target: number | null) {
    if (target == null || saving) return;
    if (await save()) nav.stepBook(target);
  }

  function onKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      void saveAndClose();
    }
  }

  // ---- Cover ----
  let coverBusy = $state(false);
  let covers = $state<{ status: 'idle' | 'loading' | 'done' | 'error'; items: CoverCandidate[]; error: string | null }>({
    status: 'idle',
    items: [],
    error: null
  });

  async function coverChange(label: string, fn: () => Promise<Book>) {
    if (coverBusy) return;
    coverBusy = true;
    try {
      const updated = await attempt(label, fn);
      if (updated) {
        library.upsert(updated);
        const note = deviceNote(updated);
        toasts.success(note ? `Cover updated. ${note}` : 'Cover updated.');
      }
    } finally {
      coverBusy = false;
    }
  }

  async function chooseImage() {
    const picked = await attempt('Could not open the file picker', () =>
      open({ multiple: false, filters: [{ name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'webp', 'gif'] }] })
    );
    if (!picked || Array.isArray(picked)) return;
    await coverChange('Could not use that image', () => setCoverFromFile(id, picked));
  }

  async function findOnline() {
    covers = { status: 'loading', items: [], error: null };
    const forId = id;
    try {
      const items = await findCovers(forId);
      if (forId === id) covers = { status: 'done', items, error: null };
    } catch (e) {
      if (forId === id) covers = { status: 'error', items: [], error: errorMessage(e) };
    }
  }

  async function useCandidate(c: CoverCandidate) {
    await coverChange('Could not use that cover', () => setCoverFromUrl(id, c.url));
  }
</script>

{#if book && form}
  <Modal labelledby="editor-h" maxWidth="1000px" onclose={close}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="editor" onkeydown={onKey}>
      <header>
        <h2 id="editor-h">Edit metadata</h2>
        <span class="pos mono">{position >= 0 ? `${position + 1} of ${siblings.length}` : ''}</span>
        <div class="steps">
          <button class="btn sm" disabled={prevId == null || saving} onclick={() => step(prevId)} title="Saves your changes, then goes to the previous book">
            <span class="flip"><Icon name="chevron" size={16} /></span>
            Previous
          </button>
          <button class="btn sm" disabled={nextId == null || saving} onclick={() => step(nextId)} title="Saves your changes, then goes to the next book">
            Next
            <Icon name="chevron" size={16} />
          </button>
        </div>
        <button class="icon-btn bare" aria-label="Close editor without saving" onclick={close}><Icon name="close" size={18} /></button>
      </header>

      <div class="cols">
        <div class="cover-col">
          <Cover title={form.title || book.title} author={form.author || book.author} coverPath={book.coverPath} />
          <div class="cover-btns">
            <button class="btn sm" onclick={chooseImage} disabled={coverBusy}>Choose image…</button>
            <button class="btn sm" onclick={findOnline} disabled={coverBusy || covers.status === 'loading'}>Find covers online</button>
            <button class="btn sm ghost" onclick={() => coverChange('Could not remove the cover', () => removeCover(id))} disabled={coverBusy || !book.coverPath}>
              Use generated cover
            </button>
          </div>

          {#if covers.status === 'loading'}
            <p class="cover-note" role="status"><span class="spin ring"></span>Looking for covers…</p>
          {:else if covers.status === 'error'}
            <p class="cover-note bad" role="alert">Could not look for covers. {covers.error}</p>
          {:else if covers.status === 'done'}
            {#if covers.items.length === 0}
              <p class="cover-note">No covers found for this title and author.</p>
            {:else}
              <div class="thumbs" role="group" aria-label="Cover options">
                {#each covers.items as c, i (c.url)}
                  <button class="thumb" onclick={() => useCandidate(c)} disabled={coverBusy} aria-label="Use cover option {i + 1}">
                    <img src={c.thumbUrl} alt="" loading="lazy" referrerpolicy="no-referrer" />
                  </button>
                {/each}
              </div>
            {/if}
          {/if}
        </div>

        <form
          class="fields"
          onsubmit={(e) => {
            e.preventDefault();
            void saveAndClose();
          }}
        >
          <div class="field">
            <label for="ed-title">Title</label>
            <input id="ed-title" class="input" bind:value={form.title} required data-autofocus />
          </div>
          <div class="field">
            <label for="ed-author">Author(s)</label>
            <input id="ed-author" class="input" bind:value={form.author} aria-describedby="ed-author-hint" />
            <span id="ed-author-hint" class="hint">Join several authors with “&amp;”, like Neil Gaiman &amp; Terry Pratchett.</span>
          </div>
          <div class="field">
            <label for="ed-sort">Author sort</label>
            <div class="inline">
              <input
                id="ed-sort"
                class="input"
                bind:value={form.authorSort}
                placeholder={form.authorSortAuto ? 'Worked out from the author when you save' : 'Surname, First name'}
                oninput={() => (form!.authorSortAuto = false)}
              />
              <button type="button" class="link-btn" onclick={() => { form!.authorSort = ''; form!.authorSortAuto = true; }}>auto</button>
            </div>
          </div>

          <div class="row">
            <div class="field grow">
              <label for="ed-series">Series</label>
              <input id="ed-series" class="input" bind:value={form.series} placeholder="Not part of a series" />
            </div>
            <div class="field num">
              <label for="ed-index">Number</label>
              <input id="ed-index" class="input" type="number" step="0.5" min="0" bind:value={form.seriesIndex} disabled={!form.series.trim()} />
            </div>
          </div>

          <div class="field">
            <span class="lbl" id="ed-rating-l">Rating</span>
            <Stars value={form.rating} size={22} label="Rating" onchange={(v) => (form!.rating = v)} />
          </div>

          <ChipInput
            id="ed-collections"
            label="Collections"
            values={form.collections}
            suggestions={labels.collections.map((l) => l.name)}
            placeholder="Add to a collection"
            onchange={(v) => (form!.collections = v)}
          />
          <ChipInput
            id="ed-tags"
            label="Tags"
            values={form.tags}
            suggestions={labels.tags.map((l) => l.name)}
            placeholder="Add a tag"
            onchange={(v) => (form!.tags = v)}
          />

          <div class="row">
            <div class="field grow">
              <label for="ed-publisher">Publisher</label>
              <input id="ed-publisher" class="input" bind:value={form.publisher} />
            </div>
            <div class="field grow">
              <label for="ed-published">Published</label>
              <input id="ed-published" class="input" bind:value={form.published} placeholder="2015 or 2015-06-30" inputmode="numeric" />
            </div>
          </div>
          <div class="row">
            <div class="field grow">
              <label for="ed-language">Language</label>
              <input id="ed-language" class="input" bind:value={form.language} list="ed-languages" placeholder="en" />
              <datalist id="ed-languages">
                {#each ['en', 'fr', 'de', 'es', 'it', 'nl', 'pt', 'ru', 'ja', 'zh'] as l (l)}<option value={l}></option>{/each}
              </datalist>
            </div>
            <div class="field grow">
              <label for="ed-isbn">ISBN</label>
              <input id="ed-isbn" class="input mono" bind:value={form.isbn} placeholder="9780000000000" />
            </div>
          </div>

          <div class="field">
            <label for="ed-desc">Description</label>
            <textarea id="ed-desc" class="input" rows="6" bind:value={form.description}></textarea>
          </div>
          <button type="submit" class="sr-only" tabindex="-1">Save</button>
        </form>
      </div>

      <footer>
        {#if problem}
          <p class="problem" role="alert">{problem}</p>
        {:else if book.onDevices.length > 0}
          <p class="hint grow">This book is on your {book.onDevices.map((d) => devices.name(d)).join(', ')}. Its copy updates on the next sync after you save.</p>
        {:else}
          <span class="grow"></span>
        {/if}
        <button class="btn" onclick={close} disabled={saving}>Cancel</button>
        <button class="btn primary" onclick={saveAndClose} disabled={saving} title="Ctrl+S">
          {saving ? 'Saving…' : dirty ? 'Save' : 'Done'}
        </button>
      </footer>
    </div>
  </Modal>
{/if}

<style>
  header {
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--surface-modal);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-soft);
  }
  h2 {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .pos {
    flex: 1;
    font-size: 12px;
    color: var(--faint);
  }
  .steps {
    display: flex;
    gap: 8px;
  }
  .flip {
    display: flex;
    transform: scaleX(-1);
  }
  .cols {
    display: flex;
    flex-wrap: wrap;
    gap: 28px;
    padding: 20px;
    align-items: flex-start;
  }
  .cover-col {
    flex: 0 1 230px;
    min-width: 180px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .cover-btns {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .cover-btns .btn {
    width: 100%;
  }
  .cover-note {
    font-size: 13px;
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cover-note.bad {
    color: var(--danger);
    word-break: break-word;
  }
  .ring {
    flex: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--border-dashed);
    border-top-color: var(--accent);
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .thumb {
    padding: 0;
    border: 1px solid var(--border-control);
    border-radius: var(--radius-s);
    background: var(--surface-sunken);
    overflow: hidden;
    aspect-ratio: 2 / 3;
  }
  .thumb:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .fields {
    flex: 999 1 400px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }
  .grow {
    flex: 1 1 160px;
  }
  .num {
    flex: 0 0 110px;
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .lbl {
    font-size: 12px;
    color: var(--muted);
  }
  .hint {
    font-size: 12px;
    color: var(--faint);
  }
  footer {
    position: sticky;
    bottom: 0;
    z-index: 2;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-footer);
  }
  .problem {
    flex: 1 1 200px;
    color: var(--danger);
    font-size: 13px;
  }
</style>
