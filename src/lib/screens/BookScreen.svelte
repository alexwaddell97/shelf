<script lang="ts">
  import {
    createLabel,
    deleteBook,
    labelBooks,
    openBook,
    openReader,
    READABLE_FORMATS,
    queueForDevice,
    removeFromDevice,
    revealBook,
    unqueue,
    type Device
  } from '$lib/api';
  import ClampText from '$lib/components/ClampText.svelte';
  import Cover from '$lib/components/Cover.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Stars from '$lib/components/Stars.svelte';
  import { authorEntries, formatBytes, formatDate, formatIndex, languageName, relativeDate, statusLabel } from '$lib/format';
  import { devices } from '$lib/stores/devices.svelte';
  import { labels } from '$lib/stores/labels.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { authorScope, collectionScope, nav, seriesScope, tagScope, type Back } from '$lib/stores/nav.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';

  let { id, back }: { id: number; back: Back } = $props();

  const book = $derived(library.byId(id));
  const connected = $derived(devices.connected);

  let confirmDelete = $state(false);
  let alsoFile = $state(false);
  let busy = $state(false);
  let collOpen = $state(false);
  let newColl = $state('');

  // Close popovers and dialogs when the page moves to a different book.
  $effect(() => {
    id;
    confirmDelete = false;
    collOpen = false;
  });

  /** Closes a popover on Escape or a click outside it. */
  function popover(node: HTMLElement, close: () => void) {
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (!node.contains(t) && !node.parentElement?.contains(t)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        close();
      }
    };
    document.addEventListener('mousedown', onDown);
    node.addEventListener('keydown', onKey);
    return {
      destroy() {
        document.removeEventListener('mousedown', onDown);
        node.removeEventListener('keydown', onKey);
      }
    };
  }

  async function toggleCollection(labelId: number, inIt: boolean) {
    await run('Could not change the collection', () => labelBooks(labelId, [id], !inIt));
  }

  async function createAndAdd() {
    const name = newColl.trim();
    if (!name) return;
    const existing = labels.collections.find((c) => c.name.toLowerCase() === name.toLowerCase());
    await run(
      'Could not add the book to a new collection',
      async () => {
        if (existing) {
          await labelBooks(existing.id, [id], true);
          return;
        }
        await createLabel('collection', name);
        await labels.refresh();
        const made = labels.collections.find((c) => c.name.toLowerCase() === name.toLowerCase());
        if (!made) throw new Error('The new collection did not show up.');
        await labelBooks(made.id, [id], true);
      },
      `Added to ${existing?.name ?? name}.`
    );
    newColl = '';
  }

  const deviceFor = (deviceId: string): Pick<Device, 'name' | 'sendFormat'> =>
    devices.byId(deviceId) ?? { name: 'a device', sendFormat: 'epub' };

  const onList = $derived(book ? book.onDevices : []);
  const queuedList = $derived(book ? book.queuedFor : []);
  const sendTargets = $derived(
    book ? devices.connected.filter((d) => !book.onDevices.includes(d.id) && !book.queuedFor.includes(d.id)) : []
  );

  const formats = $derived.by(() => {
    if (!book) return '';
    const parts = [`${book.format.toUpperCase()} on PC`];
    for (const d of onList) parts.push(`${deviceFor(d).sendFormat.toUpperCase()} on ${deviceFor(d).name}`);
    return onList.length ? parts.join(' · ') : book.format.toUpperCase();
  });

  async function run(label: string, fn: () => Promise<unknown>, success?: string) {
    if (busy) return;
    busy = true;
    try {
      const ok = await attempt(label, async () => {
        await fn();
        return true;
      });
      if (ok) {
        if (success) toasts.success(success);
        await library.refresh();
      }
    } finally {
      busy = false;
    }
  }

  const send = (d: Device) =>
    run(
      `Could not queue the book for ${d.name}`,
      () => queueForDevice(d.id, [id]),
      `Queued. It goes to your ${d.name} as ${d.sendFormat.toUpperCase()} on the next sync.`
    );
  const cancelSend = (deviceId: string) =>
    run('Could not cancel sending', () => unqueue(deviceId, id), 'Removed from the send queue.');
  const removeFrom = (d: Device) =>
    run(
      `Could not remove the book from ${d.name}`,
      () => removeFromDevice(d.id, id),
      `Removed from your ${d.name}. It is still in your library.`
    );
  // Shelf's own reader for the formats it can show; another app for the rest.
  const read = () =>
    book && READABLE_FORMATS.includes(book.format)
      ? run('Could not open the reader', () => openReader(id))
      : run('Could not open the book', () => openBook(id));
  const openElsewhere = () => run('Could not open the book', () => openBook(id));
  const reveal = () => run('Could not show the file', () => revealBook(id));

  async function confirmedDelete() {
    if (!book || busy) return;
    busy = true;
    const title = book.title;
    try {
      const ok = await attempt('Could not delete the book', async () => {
        await deleteBook(id, alsoFile);
        return true;
      });
      if (ok) {
        confirmDelete = false;
        library.remove(id);
        toasts.success(`Deleted “${title}”${alsoFile ? ' and its file' : ''}.`);
        nav.go('library');
        void library.refresh();
      }
    } finally {
      busy = false;
    }
  }
</script>

<section aria-labelledby="book-h">
  <button class="btn back" onclick={() => nav.goBack(back)}>
    <Icon name="back" />
    {nav.backLabel(back)}
  </button>

  {#if !book}
    <h1 id="book-h" class="page-title">{library.loaded ? 'Book not found' : 'Loading…'}</h1>
    {#if library.loaded}
      <p class="page-sub">This book is no longer in your library.</p>
    {/if}
  {:else}
    <div class="layout">
      <div class="cover-col">
        <Cover title={book.title} author={book.author} coverPath={book.coverPath} />
      </div>

      <div class="info">
        <div>
          {#if book.series}
            <button class="series mono" onclick={() => nav.showScope(seriesScope(book.series!))} title="Show the whole series">
              {book.series}{book.seriesIndex != null ? ` · Book ${formatIndex(book.seriesIndex)}` : ''}
            </button>
          {/if}
          <h1 id="book-h" class="title">{book.title}</h1>
          <p class="author">
            {#each authorEntries(book) as a, i (a.key)}
              {#if i > 0}<span class="amp"> &amp; </span>{/if}<button class="author-link" onclick={() => nav.showScope(authorScope(a.key, a.name))}>{a.name}</button>
            {/each}
          </p>
          {#if book.rating}
            <div class="rating"><Stars value={book.rating} size={18} /></div>
          {/if}
        </div>
        {#if book.description}
          <div class="desc">
            {#key book.id}<ClampText text={book.description} lines={4} />{/key}
          </div>
        {:else}
          <p class="desc none">No description yet. Edit details to add one.</p>
        {/if}

        <div class="chips">
          {#each book.collections as c (c)}
            <button class="chip coll" onclick={() => nav.showScope(collectionScope(c))} title="Show this collection">
              <Icon name="bookmark" size={13} />{c}
            </button>
          {/each}
          {#each book.tags as t (t)}
            <button class="chip" onclick={() => nav.showScope(tagScope(t))} title="Show books with this tag">
              <Icon name="tag" size={13} />{t}
            </button>
          {/each}
          <div class="popwrap">
            <button class="chip add" aria-expanded={collOpen} aria-haspopup="dialog" onclick={() => (collOpen = !collOpen)}>
              <Icon name="plus" size={13} />Add to collection
            </button>
            {#if collOpen}
              <div class="pop" role="dialog" aria-label="Add to a collection" tabindex="-1" use:popover={() => (collOpen = false)}>
                {#each labels.collections as c (c.id)}
                  {@const inIt = book.collections.includes(c.name)}
                  <button class="pop-item" role="menuitemcheckbox" aria-checked={inIt} disabled={busy} onclick={() => toggleCollection(c.id, inIt)}>
                    <span class="tick">{#if inIt}<Icon name="check" size={14} />{/if}</span>
                    {c.name}
                  </button>
                {:else}
                  <p class="pop-empty">No collections yet.</p>
                {/each}
                <form class="pop-new" onsubmit={(e) => { e.preventDefault(); void createAndAdd(); }}>
                  <label class="sr-only" for="new-coll">New collection name</label>
                  <input id="new-coll" class="input" placeholder="New collection…" bind:value={newColl} autocomplete="off" />
                  <button class="btn sm primary" type="submit" disabled={busy || !newColl.trim()}>Add</button>
                </form>
              </div>
            {/if}
          </div>
        </div>

        {#each onList as deviceId (deviceId)}
          <div class="status on">
            <div class="status-head">
              <span class="status-title"><span class="dot"></span>On your {deviceFor(deviceId).name}</span>
              <span class="mono small">{book.lastRead ? `Last read ${relativeDate(book.lastRead)}` : 'Not opened yet'}</span>
            </div>
            <div class="bar"><div style:width="{book.progress}%"></div></div>
            <div class="small-soft">{statusLabel(book)} · progress synced from the device</div>
          </div>
        {/each}
        {#each queuedList as deviceId (deviceId)}
          <div class="status queued">
            {#if onList.includes(deviceId)}
              <div class="status-title amber">Waiting to update on {deviceFor(deviceId).name}</div>
              <div class="small-soft">The copy there is refreshed the next time you sync.</div>
            {:else}
              <div class="status-title amber">Waiting to send to {deviceFor(deviceId).name}</div>
              <div class="small-soft">
                Goes over as {deviceFor(deviceId).sendFormat.toUpperCase()} the next time you sync.
              </div>
            {/if}
          </div>
        {/each}
        {#if onList.length === 0 && queuedList.length === 0}
          <div class="status pc">
            <div class="status-title">Only on this PC</div>
            <div class="small-soft">
              {#if connected.length}
                Send it to your {connected[0].name} and Shelf converts it to {connected[0].sendFormat.toUpperCase()} on the way.
              {:else}
                Plug in an e-reader and you can send it from here.
              {/if}
            </div>
          </div>
        {/if}

        <div class="actions">
          {#each sendTargets as d, i (d.id)}
            <button
              class="btn"
              class:primary={i === 0}
              disabled={busy}
              onclick={() => send(d)}
            >
              <Icon name="device-send" />
              Send to {d.name}
            </button>
          {/each}
          <button class="btn" disabled={busy} onclick={read}>Read</button>
          <button class="btn" onclick={() => (nav.editing = true)}>Edit details</button>
          <button class="btn" disabled={busy} onclick={reveal}>Show file</button>
          <button class="btn" disabled={busy} onclick={openElsewhere}>Open in another app</button>
          {#each queuedList as deviceId (deviceId)}
            <button class="btn" disabled={busy} onclick={() => cancelSend(deviceId)}>
              {onList.includes(deviceId) ? 'Cancel update on' : 'Cancel send to'} {deviceFor(deviceId).name}
            </button>
          {/each}
          {#each onList as deviceId (deviceId)}
            {@const d = devices.byId(deviceId)}
            {#if d?.connected}
              <button class="btn danger" disabled={busy} onclick={() => removeFrom(d)}>Remove from {d.name}</button>
            {/if}
          {/each}
          <button
            class="btn danger"
            disabled={busy}
            onclick={() => {
              alsoFile = false;
              confirmDelete = true;
            }}
          >
            Delete from library
          </button>
        </div>

        <dl>
          <div><dt>Formats</dt><dd>{formats}</dd></div>
          <div><dt>File size</dt><dd>{formatBytes(book.sizeBytes) || 'Unknown'}</dd></div>
          <div><dt>Added</dt><dd>{formatDate(book.addedAt) || 'Unknown'}</dd></div>
          <div><dt>Status</dt><dd>{statusLabel(book)}</dd></div>
          {#if book.publisher}<div><dt>Publisher</dt><dd>{book.publisher}</dd></div>{/if}
          {#if book.published}<div><dt>Published</dt><dd>{book.published}</dd></div>{/if}
          {#if languageName(book.language)}<div><dt>Language</dt><dd>{languageName(book.language)}</dd></div>{/if}
          {#if book.isbn}<div><dt>ISBN</dt><dd class="mono">{book.isbn}</dd></div>{/if}
        </dl>
      </div>
    </div>
  {/if}
</section>

{#if confirmDelete && book}
  <Modal labelledby="delete-h" maxWidth="460px" onclose={() => (confirmDelete = false)}>
    <div class="dialog-body">
      <h2 id="delete-h">Delete “{book.title}”?</h2>
      <p class="muted">
        It leaves your library and any send queue. Copies already on an e-reader stay there until you remove them.
      </p>
      <label class="check">
        <input type="checkbox" bind:checked={alsoFile} />
        <span>Also delete the file from this PC</span>
      </label>
      <div class="dialog-actions">
        <button class="btn" data-autofocus onclick={() => (confirmDelete = false)} disabled={busy}>Cancel</button>
        <button class="btn destructive" onclick={confirmedDelete} disabled={busy}>
          {busy ? 'Deleting…' : alsoFile ? 'Delete book and file' : 'Delete from library'}
        </button>
      </div>
    </div>
  </Modal>
{/if}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .layout {
    display: flex;
    flex-wrap: wrap;
    gap: 44px;
    align-items: flex-start;
  }
  .cover-col {
    flex: 0 1 300px;
    min-width: 200px;
  }
  .info {
    flex: 999 1 380px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .series {
    display: block;
    padding: 0;
    border: 0;
    background: none;
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent);
    text-align: left;
  }
  .series:hover,
  .author-link:hover {
    text-decoration: underline;
  }
  .author-link {
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
  }
  .rating {
    margin-top: 10px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-soft);
    font-size: 13px;
  }
  .chip:hover {
    border-color: var(--border-dashed);
  }
  .chip.coll {
    color: var(--accent);
  }
  .chip.add {
    border-style: dashed;
    background: transparent;
    color: var(--muted);
  }
  .popwrap {
    position: relative;
  }
  .pop {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 6;
    width: 260px;
    max-height: 320px;
    overflow-y: auto;
    padding: 6px;
    border-radius: var(--radius-l);
    background: var(--surface-raised);
    border: 1px solid var(--border-control);
    box-shadow: var(--shadow-2);
  }
  .pop-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-m);
    background: transparent;
    color: var(--text);
    text-align: left;
  }
  .pop-item:hover {
    background: var(--surface-hover);
  }
  .tick {
    display: inline-flex;
    width: 16px;
    color: var(--teal);
  }
  .pop-empty {
    padding: 8px 10px;
    color: var(--muted);
    font-size: 13px;
  }
  .pop-new {
    display: flex;
    gap: 6px;
    padding: 6px 4px 2px;
    margin-top: 4px;
    border-top: 1px solid var(--border-soft);
  }
  .pop-new .input {
    min-width: 0;
  }
  .title {
    margin-top: 6px;
    font-family: var(--font-serif);
    font-variant-numeric: lining-nums;
    font-size: 42px;
    line-height: 1.05;
    font-weight: 600;
    letter-spacing: -0.018em;
    overflow-wrap: anywhere;
  }
  .author {
    margin-top: 8px;
    font-size: 18px;
    color: var(--muted);
  }
  .desc {
    max-width: 60ch;
    font-size: 16px;
    color: var(--text-body);
    white-space: pre-line;
  }
  .desc.none {
    color: var(--faint);
    font-size: 14px;
  }
  .status {
    max-width: 560px;
    padding: 16px;
    border-radius: var(--r2);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .status.on {
    background: var(--teal-bg);
    border: 1px solid var(--teal-border);
  }
  .status.queued {
    gap: 4px;
    background: var(--accent-bg);
    border: 1px solid var(--accent-border);
    color: var(--accent-text);
  }
  .status.pc {
    gap: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text-soft);
  }
  .status-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .status-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .status.on .status-title {
    color: var(--teal);
  }
  .status.pc .status-title {
    color: var(--text);
  }
  .status-title.amber {
    color: var(--accent);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--teal);
  }
  .small {
    font-size: 12px;
    color: var(--muted);
  }
  .small-soft {
    font-size: 13px;
    color: var(--text-soft);
  }
  .status.queued .small-soft {
    color: inherit;
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--teal-track);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--teal);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  dl {
    max-width: 560px;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px 24px;
    padding-top: 18px;
    border-top: 1px solid var(--border-soft);
  }
  dt {
    font-size: 12px;
    color: var(--faint);
  }
  dd {
    margin: 2px 0 0;
    overflow-wrap: anywhere;
  }

  .dialog-body {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .dialog-body h2 {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
    overflow-wrap: anywhere;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius-l);
    background: var(--surface-raised);
    border: 1px solid var(--border);
    cursor: pointer;
  }
  .check input {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .dialog-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
  }
  .btn.destructive {
    border-color: transparent;
    background: var(--danger);
    color: var(--on-danger);
    font-weight: 600;
  }
</style>
