<script lang="ts">
  import { updateBooks, type Book, type BookPatch } from '$lib/api';
  import { plural } from '$lib/format';
  import { labels } from '$lib/stores/labels.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';
  import ChipInput from './ChipInput.svelte';
  import Modal from './Modal.svelte';
  import Stars from './Stars.svelte';

  // Edit many books at once. Each change is off until you turn it on, so only what you
  // touch is changed; tags and collections are added or removed, never replaced wholesale.

  interface Props {
    /** The selected books, in the order the list shows them (series numbers follow it). */
    ids: number[];
    onclose: () => void;
  }

  let { ids, onclose }: Props = $props();

  const books = $derived(ids.map((id) => library.byId(id)).filter((b): b is Book => !!b));

  let setAuthor = $state(false);
  let author = $state('');
  let setSeries = $state(false);
  let series = $state('');
  let number = $state(true);
  let firstNumber = $state(1);
  let setRating = $state(false);
  let rating = $state<number | null>(null);
  let addTags = $state<string[]>([]);
  let removeTags = $state<string[]>([]);
  let addCollections = $state<string[]>([]);
  let removeCollections = $state<string[]>([]);
  let saving = $state(false);

  // Start from what the selection already shares, so a small fix is quick.
  const shared = (pick: (b: Book) => string | null) => {
    const values = new Set(books.map((b) => pick(b) ?? ''));
    return values.size === 1 ? [...values][0] : '';
  };
  $effect.pre(() => {
    author ||= shared((b) => b.author);
    series ||= shared((b) => b.series);
  });

  const presentTags = $derived([...new Set(books.flatMap((b) => b.tags))].sort());
  const presentCollections = $derived([...new Set(books.flatMap((b) => b.collections))].sort());

  const anything = $derived(
    setAuthor || setSeries || setRating || addTags.length + removeTags.length + addCollections.length + removeCollections.length > 0
  );

  const merge = (current: string[], add: string[], remove: string[]) => {
    const gone = new Set(remove.map((r) => r.toLowerCase()));
    const out = current.filter((c) => !gone.has(c.toLowerCase()));
    for (const a of add) if (!out.some((o) => o.toLowerCase() === a.toLowerCase())) out.push(a);
    return out;
  };
  const sameList = (a: string[], b: string[]) => a.length === b.length && a.every((x, i) => x === b[i]);

  async function apply() {
    const name = author
      .split('&')
      .map((s) => s.trim())
      .filter(Boolean)
      .join(' & ');
    if (setAuthor && !name) return toasts.error('Type an author, or turn off "Set the author".');
    const seriesName = series.trim();
    const updates = books.map((b, i) => {
      const patch: BookPatch = {};
      if (setAuthor && name !== b.author) {
        patch.author = name;
        patch.authorSort = null;
      }
      if (setSeries) {
        patch.series = seriesName || null;
        if (!seriesName) patch.seriesIndex = null;
        else if (number) patch.seriesIndex = Number(firstNumber) + i;
      }
      if (setRating) patch.rating = rating;
      const tags = merge(b.tags, addTags, removeTags);
      if (!sameList(tags, b.tags)) patch.tags = tags;
      const collections = merge(b.collections, addCollections, removeCollections);
      if (!sameList(collections, b.collections)) patch.collections = collections;
      return { id: b.id, patch };
    });
    saving = true;
    try {
      const changed = await attempt('Could not save the changes', () => updateBooks(updates.filter((u) => Object.keys(u.patch).length)));
      if (changed !== undefined) {
        toasts.success(changed ? `Updated ${plural(changed, 'book')}.` : 'Nothing needed changing.');
        await library.refresh();
        void labels.refresh();
        onclose();
      }
    } finally {
      saving = false;
    }
  }
</script>

<Modal labelledby="bulk-h" maxWidth="620px" {onclose}>
  <div class="bulk">
    <header>
      <h2 id="bulk-h">Edit {plural(books.length, 'book')}</h2>
      <p>Turn on what you want to change. Everything else stays as it is.</p>
    </header>

    <div class="rows">
      <div class="row" class:on={setAuthor}>
        <label class="toggle"><input type="checkbox" bind:checked={setAuthor} /> Set the author</label>
        {#if setAuthor}
          <input class="input" bind:value={author} placeholder="Author (several: A & B)" aria-label="Author" />
        {/if}
      </div>

      <div class="row" class:on={setSeries}>
        <label class="toggle"><input type="checkbox" bind:checked={setSeries} /> Set the series</label>
        {#if setSeries}
          <input class="input" bind:value={series} placeholder="Series name (empty removes them from a series)" aria-label="Series" />
          {#if series.trim()}
            <label class="sub">
              <input type="checkbox" bind:checked={number} />
              Number them in list order, starting at
              <input class="input num" type="number" min="0" step="1" bind:value={firstNumber} aria-label="First number" disabled={!number} />
            </label>
          {/if}
        {/if}
      </div>

      <div class="row" class:on={setRating}>
        <label class="toggle"><input type="checkbox" bind:checked={setRating} /> Set the rating</label>
        {#if setRating}
          <div class="stars"><Stars value={rating} onchange={(v) => (rating = v)} label="Rating" /></div>
        {/if}
      </div>

      <div class="pair">
        <ChipInput id="bulk-add-tags" label="Add tags" values={addTags} suggestions={labels.tags.map((l) => l.name)} placeholder="Add a tag" onchange={(v) => (addTags = v)} />
        <ChipInput id="bulk-remove-tags" label="Remove tags" values={removeTags} suggestions={presentTags} placeholder="Tag to remove" onchange={(v) => (removeTags = v)} />
      </div>
      <div class="pair">
        <ChipInput id="bulk-add-coll" label="Add to collections" values={addCollections} suggestions={labels.collections.map((l) => l.name)} placeholder="Add to a collection" onchange={(v) => (addCollections = v)} />
        <ChipInput id="bulk-remove-coll" label="Remove from collections" values={removeCollections} suggestions={presentCollections} placeholder="Collection to leave" onchange={(v) => (removeCollections = v)} />
      </div>
    </div>

    <footer>
      <span class="grow">Books on your e-reader get the changes on the next sync.</span>
      <button class="btn" onclick={onclose} disabled={saving}>Cancel</button>
      <button class="btn primary" onclick={apply} disabled={!anything || saving}>{saving ? 'Saving…' : `Change ${plural(books.length, 'book')}`}</button>
    </footer>
  </div>
</Modal>

<style>
  .bulk {
    display: flex;
    flex-direction: column;
    background: var(--surface-modal);
  }
  header {
    padding: 20px 22px 4px;
  }
  h2 {
    font-size: 20px;
    font-weight: 700;
  }
  header p {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--muted);
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 22px 20px;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--r2);
    background: var(--fill);
  }
  .row.on {
    background: var(--accent-bg);
  }
  .toggle,
  .sub {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .sub {
    font-weight: 400;
    color: var(--text-soft);
  }
  .num {
    width: 72px;
    height: 32px;
  }
  .stars {
    display: flex;
  }
  .pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  @media (max-width: 640px) {
    .pair {
      grid-template-columns: 1fr;
    }
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 22px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-footer);
  }
  .grow {
    flex: 1 1 200px;
    font-size: 12px;
    color: var(--muted);
  }
</style>
