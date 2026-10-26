<script lang="ts">
  import { createLabel, deleteLabel, renameLabel, type Label } from '$lib/api';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';
  import Cover from '$lib/components/Cover.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { authorEntries, plural, sortTitle } from '$lib/format';
  import { labels } from '$lib/stores/labels.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { authorScope, collectionScope, nav, seriesScope, tagScope, type BrowseKind } from '$lib/stores/nav.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';
  import type { Book } from '$lib/api';

  let { kind }: { kind: BrowseKind } = $props();

  const TITLES: Record<BrowseKind, [string, string]> = {
    authors: ['Authors', 'Everyone in your library, sorted by surname.'],
    series: ['Series', 'Books that belong together, in reading order.'],
    collections: ['Collections', 'Your own groups of books. Make one for a trip, a book club or a mood.'],
    tags: ['Tags', 'Descriptive tags from your books and from calibre.']
  };

  // ---- Authors ----
  const authors = $derived.by(() => {
    const map = new Map<string, { key: string; name: string; count: number }>();
    for (const b of library.books) {
      for (const e of authorEntries(b)) {
        const hit = map.get(e.key);
        if (hit) hit.count++;
        else map.set(e.key, { ...e, count: 1 });
      }
    }
    return [...map.values()].sort((a, b) => a.key.localeCompare(b.key));
  });
  const authorGroups = $derived.by(() => {
    const groups: { letter: string; items: typeof authors }[] = [];
    for (const a of authors) {
      const ch = a.key.charAt(0).toUpperCase();
      const letter = /\p{L}/u.test(ch) ? ch : '#';
      const last = groups[groups.length - 1];
      if (last && last.letter === letter) last.items.push(a);
      else groups.push({ letter, items: [a] });
    }
    return groups;
  });

  // ---- Series ----
  const seriesList = $derived.by(() => {
    const map = new Map<string, Book[]>();
    for (const b of library.books) {
      if (!b.series) continue;
      const list = map.get(b.series);
      if (list) list.push(b);
      else map.set(b.series, [b]);
    }
    return [...map.entries()]
      .map(([name, books]) => {
        const sorted = [...books].sort(
          (a, b) => (a.seriesIndex ?? Infinity) - (b.seriesIndex ?? Infinity) || sortTitle(a.title).localeCompare(sortTitle(b.title))
        );
        return { name, count: books.length, first: sorted[0] };
      })
      .sort((a, b) => sortTitle(a.name).localeCompare(sortTitle(b.name)));
  });

  // ---- Collections and tags ----
  const isCollections = $derived(kind === 'collections');
  const items = $derived(isCollections ? labels.collections : labels.tags);

  let newName = $state('');
  let problem = $state<string | null>(null);
  let renamingId = $state<number | null>(null);
  let renameText = $state('');
  let deleting = $state<Label | null>(null);
  let busy = $state(false);

  async function afterChange() {
    await Promise.all([labels.refresh(), library.refresh()]);
  }

  async function create(e: SubmitEvent) {
    e.preventDefault();
    const name = newName.trim();
    problem = null;
    if (!name) {
      problem = 'Give the collection a name.';
      return;
    }
    if (labels.collections.some((c) => c.name.toLowerCase() === name.toLowerCase())) {
      problem = `You already have a collection called ${name}.`;
      return;
    }
    busy = true;
    try {
      const ok = await attempt('Could not create the collection', async () => {
        await createLabel('collection', name);
        return true;
      });
      if (ok) {
        newName = '';
        toasts.success(`Created the collection ${name}.`);
        await labels.refresh();
      }
    } finally {
      busy = false;
    }
  }

  function startRename(l: Label) {
    renamingId = l.id;
    renameText = l.name;
  }

  async function saveRename(l: Label) {
    const name = renameText.trim();
    if (!name || name === l.name) {
      renamingId = null;
      return;
    }
    if (items.some((x) => x.id !== l.id && x.name.toLowerCase() === name.toLowerCase())) {
      toasts.error(`There is already one called ${name}.`);
      return;
    }
    busy = true;
    try {
      const ok = await attempt(`Could not rename ${l.name}`, async () => {
        await renameLabel(l.id, name);
        return true;
      });
      if (ok) {
        renamingId = null;
        await afterChange();
      }
    } finally {
      busy = false;
    }
  }

  async function confirmDelete() {
    const l = deleting;
    if (!l || busy) return;
    busy = true;
    try {
      const ok = await attempt(`Could not delete ${l.name}`, async () => {
        await deleteLabel(l.id);
        return true;
      });
      if (ok) {
        deleting = null;
        toasts.success(`Deleted ${l.name}. Your books are untouched.`);
        await afterChange();
      }
    } finally {
      busy = false;
    }
  }

  const open = (l: Label) => nav.showScope(isCollections ? collectionScope(l.name) : tagScope(l.name));
</script>

<section aria-labelledby="browse-h">
  <div>
    <h1 id="browse-h" class="page-title">{TITLES[kind][0]}</h1>
    <p class="page-sub">{TITLES[kind][1]}</p>
  </div>

  {#if kind === 'authors'}
    {#if !library.loaded}
      <p class="muted">Loading…</p>
    {:else if authors.length === 0}
      <p class="empty">No authors yet. Add some books first.</p>
    {:else}
      {#each authorGroups as g (g.letter)}
        <div class="group">
          <h2 class="eyebrow">{g.letter}</h2>
          <div class="rows">
            {#each g.items as a (a.key)}
              <button class="row" onclick={() => nav.showScope(authorScope(a.key, a.name))}>
                <span class="row-text">
                  <span class="row-name">{a.name}</span>
                  {#if a.key !== a.name}<span class="row-sub mono">{a.key}</span>{/if}
                </span>
                <span class="count mono">{plural(a.count, 'book')}</span>
                <Icon name="chevron" size={16} />
              </button>
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  {:else if kind === 'series'}
    {#if !library.loaded}
      <p class="muted">Loading…</p>
    {:else if seriesList.length === 0}
      <p class="empty">No series yet. Books with a series name in their details show up here.</p>
    {:else}
      <div class="book-grid">
        {#each seriesList as s (s.name)}
          <button class="series-card" onclick={() => nav.showScope(seriesScope(s.name))} aria-label="{s.name}, {plural(s.count, 'book')}">
            <span class="cover-wrap">
              <Cover title={s.first.title} author={s.first.author} coverPath={s.first.coverPath} />
            </span>
            <span class="series-name">{s.name}</span>
            <span class="row-sub mono">{plural(s.count, 'book')}</span>
          </button>
        {/each}
      </div>
    {/if}
  {:else}
    {#if isCollections}
      <form class="create" onsubmit={create}>
        <div class="field grow">
          <label for="new-collection">New collection</label>
          <input id="new-collection" class="input" bind:value={newName} placeholder="Summer reading" autocomplete="off" />
        </div>
        <button type="submit" class="btn primary sm" disabled={busy}>Create</button>
        {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
      </form>
    {/if}

    {#if !labels.loaded}
      <p class="muted">Loading…</p>
    {:else if items.length === 0}
      <p class="empty">
        {isCollections ? 'No collections yet. Create one above, then add books from a book page.' : 'No tags yet. Add tags in a book’s details.'}
      </p>
    {:else}
      <div class="rows">
        {#each items as l (l.id)}
          <div class="row label-row">
            {#if renamingId === l.id}
              <form
                class="rename"
                onsubmit={(e) => {
                  e.preventDefault();
                  void saveRename(l);
                }}
              >
                <label class="sr-only" for="rename-{l.id}">New name for {l.name}</label>
                <input
                  id="rename-{l.id}"
                  class="input"
                  bind:value={renameText}
                  data-autofocus
                  onkeydown={(e) => e.key === 'Escape' && (renamingId = null)}
                />
                <button type="submit" class="btn sm primary" disabled={busy}>Save</button>
                <button type="button" class="btn sm" onclick={() => (renamingId = null)}>Cancel</button>
              </form>
            {:else}
              <button class="open" onclick={() => open(l)}>
                <span class="row-name">{l.name}</span>
                <span class="count mono">{plural(l.count, 'book')}</span>
              </button>
              <button class="btn sm ghost" onclick={() => startRename(l)} aria-label="Rename {l.name}">Rename</button>
              <button class="btn sm ghost danger" onclick={() => (deleting = l)} aria-label="Delete {l.name}">Delete</button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</section>

{#if deleting}
  <ConfirmModal
    title="Delete “{deleting.name}”?"
    confirmLabel="Delete"
    {busy}
    onconfirm={confirmDelete}
    oncancel={() => (deleting = null)}
  >
    It is removed from {plural(deleting.count, 'book')}. The books themselves stay in your library.
  </ConfirmModal>
{/if}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .rows {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-l);
    background: var(--surface-card);
    border: 1px solid var(--border-soft);
    color: var(--text);
    text-align: left;
    transition: box-shadow var(--dur) var(--ease);
  }
  button.row:hover {
    box-shadow: inset 0 0 0 999px var(--hover-wash);
  }
  .row-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .row-name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-sub {
    font-size: 11px;
    color: var(--faint);
  }
  .count {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }
  .series-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
  }
  .cover-wrap {
    display: block;
    transition: transform var(--dur) var(--ease);
  }
  .series-card:hover .cover-wrap,
  .series-card:focus-visible .cover-wrap {
    transform: translateY(-4px);
  }
  .series-name {
    margin-top: 4px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .create {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px;
    max-width: 520px;
  }
  .grow {
    flex: 1 1 240px;
  }
  .problem {
    flex: 1 1 100%;
    color: var(--danger);
    font-size: 13px;
  }
  .rows:has(.label-row) {
    grid-template-columns: 1fr;
    max-width: 720px;
  }
  .label-row {
    padding: 6px 8px 6px 6px;
  }
  .open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 36px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-m);
    background: transparent;
    color: var(--text);
    text-align: left;
  }
  .open:hover {
    background: var(--surface-hover);
  }
  .rename {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .rename .input {
    flex: 1 1 200px;
    width: auto;
  }

  @media (prefers-reduced-motion: reduce) {
    .cover-wrap {
      transition: none;
    }
    .series-card:hover .cover-wrap,
    .series-card:focus-visible .cover-wrap {
      transform: none;
    }
  }
</style>
