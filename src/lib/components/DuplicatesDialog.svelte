<script lang="ts">
  import { findDuplicates, ignoreDuplicates, mergeDuplicates, type Book } from '$lib/api';
  import { devices } from '$lib/stores/devices.svelte';
  import { formatBytes, formatDate, plural, statusLabel } from '$lib/format';
  import { library } from '$lib/stores/library.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';
  import Cover from './Cover.svelte';
  import Modal from './Modal.svelte';

  // Books that look like the same work (same ISBN, or the same title and author). For each
  // group you pick the copy to keep; the others are merged into it and removed.

  let { onclose }: { onclose: () => void } = $props();

  let groups = $state<Book[][] | null>(null);
  let keep = $state<Record<number, number>>({});
  let busy = $state<number | null>(null);

  async function load() {
    const found = await attempt('Could not look for duplicates', findDuplicates);
    if (!found) return onclose();
    groups = found;
    // The first copy in each group is the suggested one to keep.
    keep = Object.fromEntries(found.map((g, i) => [i, g[0].id]));
  }
  void load();

  function done(i: number) {
    groups = groups?.filter((_, j) => j !== i) ?? null;
    keep = Object.fromEntries((groups ?? []).map((g, j) => [j, keep[j >= i ? j + 1 : j] ?? g[0].id]));
  }

  async function merge(i: number) {
    const g = groups?.[i];
    if (!g) return;
    const kept = keep[i] ?? g[0].id;
    busy = i;
    try {
      const book = await attempt('Could not merge these copies', () => mergeDuplicates(kept, g.map((b) => b.id).filter((id) => id !== kept)));
      if (book) {
        toasts.success(`Merged into “${book.title}”.`);
        done(i);
        void library.refresh();
      }
    } finally {
      busy = null;
    }
  }

  async function notDuplicates(i: number) {
    const g = groups?.[i];
    if (!g) return;
    const ok = await attempt('Could not save that', async () => {
      await ignoreDuplicates(g.map((b) => b.id));
      return true;
    });
    if (ok) done(i);
  }

  const where = (b: Book) => (b.onDevices.length ? `On ${devices.name(b.onDevices[0])}` : 'Only on this PC');
</script>

<Modal labelledby="dup-h" maxWidth="860px" {onclose}>
  <div class="dups">
    <header>
      <h2 id="dup-h">Duplicates</h2>
      <p>
        {#if groups === null}
          Looking through your library…
        {:else if groups.length === 0}
          No duplicates found.
        {:else}
          {plural(groups.length, 'book')} {groups.length === 1 ? 'has' : 'have'} more than one copy. Pick the one to keep: it takes on the others'
          tags, collections, reading progress and any details it's missing.
        {/if}
      </p>
    </header>

    <div class="list">
      {#each groups ?? [] as g, i (g.map((b) => b.id).join('-'))}
        <section class="group" aria-label="Copies of {g[0].title}">
          <div class="copies" role="radiogroup" aria-label="Copy to keep">
            {#each g as b (b.id)}
              {@const on = keep[i] === b.id}
              <label class="copy" class:on>
                <input type="radio" name="keep-{i}" checked={on} onchange={() => (keep[i] = b.id)} />
                <span class="thumb"><Cover title={b.title} author={b.author} coverPath={b.coverPath} /></span>
                <span class="info">
                  <strong>{b.title}</strong>
                  <span>{b.author}</span>
                  <span class="meta mono">{b.format.toUpperCase()} · {formatBytes(b.sizeBytes)} · added {formatDate(b.addedAt)}</span>
                  <span class="meta">{where(b)} · {statusLabel(b)}</span>
                  {#if on}<span class="keep-tag">Keep</span>{/if}
                </span>
              </label>
            {/each}
          </div>
          <div class="actions">
            {#if g.some((b) => b.onDevices.length) && g.filter((b) => b.onDevices.length).length > 1}
              <span class="note">Both are on your e-reader: the extra copy stays there until you remove it.</span>
            {/if}
            <span class="grow"></span>
            <button class="btn sm ghost" onclick={() => notDuplicates(i)} disabled={busy !== null}>Not duplicates</button>
            <button class="btn sm primary" onclick={() => merge(i)} disabled={busy !== null}>
              {busy === i ? 'Merging…' : `Merge ${g.length} into one`}
            </button>
          </div>
        </section>
      {/each}
    </div>

    <footer>
      <span class="grow">Merging removes the other copies from your library and deletes their files from Shelf's library folder.</span>
      <button class="btn" onclick={onclose}>Done</button>
    </footer>
  </div>
</Modal>

<style>
  .dups {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 32px);
    background: var(--surface-modal);
  }
  header {
    flex: none;
    padding: 20px 22px 12px;
    border-bottom: 1px solid var(--border-soft);
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
  .list {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 22px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-radius: var(--r2);
    background: var(--fill);
  }
  .copies {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 10px;
  }
  .copy {
    position: relative;
    display: flex;
    gap: 12px;
    padding: 10px;
    border-radius: 12px;
    background: var(--surface);
    box-shadow: 0 0 0 1px var(--border-soft);
    cursor: pointer;
  }
  .copy.on {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .copy input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .copy:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .thumb {
    flex: none;
    width: 52px;
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    font-size: 13px;
  }
  .info strong,
  .info span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .info > span:not(.keep-tag) {
    color: var(--muted);
  }
  .meta {
    font-size: 11.5px;
  }
  .keep-tag {
    align-self: flex-start;
    margin-top: 4px;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 11px;
    font-weight: 700;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .note {
    font-size: 12px;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  footer {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 14px 22px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-footer);
  }
  footer .grow {
    font-size: 12px;
    color: var(--muted);
  }
</style>
