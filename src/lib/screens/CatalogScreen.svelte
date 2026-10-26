<script lang="ts">
  import { downloadFree, downloadsDir, openUrl, sourceUrl, type CatalogBook, type Source } from '$lib/api';
  import Cover from '$lib/components/Cover.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { arrivals } from '$lib/stores/arrivals.svelte';
  import { imports } from '$lib/stores/imports.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { nav, type Back } from '$lib/stores/nav.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';
  import { wishlist } from '$lib/stores/wishlist.svelte';

  let { book, back }: { book: CatalogBook; back: Back } = $props();

  const wish = $derived(wishlist.find(book));
  const searchText = $derived(`${book.title} ${book.author}`.trim());
  const store = $derived(settings.enabledSources('store'));
  const custom = $derived(settings.enabledSources('custom'));
  const hasSources = $derived(store.length + custom.length > 0 || book.isFree);
  const freeName = $derived(book.key.startsWith('gutenberg:') ? 'Project Gutenberg' : 'Free EPUB');

  let adding = $state(false);
  let wishBusy = $state(false);

  async function addFree() {
    if (adding) return;
    adding = true;
    try {
      const id = await attempt(`Could not add “${book.title}” to your library`, () => downloadFree($state.snapshot(book)));
      if (id === undefined) return;
      toasts.success(`Added “${book.title}” to your library.`);
      await library.refresh();
      if (library.byId(id)) nav.openBook(id, { kind: 'library' });
    } finally {
      adding = false;
    }
  }

  async function openSource(src: Source) {
    const url = sourceUrl(src, searchText);
    const ok = await attempt(`Could not open ${src.name}`, async () => {
      await openUrl(url);
      return true;
    });
    if (ok) {
      // Wait for the download, so this page can turn into the book when it arrives.
      arrivals.expect($state.snapshot(book), src.name);
      toasts.info(`Opened ${src.name} in your browser`, url);
    }
  }

  const waiting = $derived(arrivals.waitingFor(book));
  // Without a watched folder, Shelf won't notice the download by itself.
  const watching = $derived((settings.value?.watchFolders.length ?? 0) > 0);

  async function watchDownloads() {
    const dir = await attempt('Could not find your Downloads folder', downloadsDir);
    if (!dir || !settings.value) return;
    if (await settings.save({ watchFolders: [...settings.value.watchFolders, dir] }))
      toasts.success('Shelf is watching your Downloads folder', dir);
  }

  async function toggleWish() {
    if (wishBusy) return;
    wishBusy = true;
    try {
      if (wish) await wishlist.remove(wish.id);
      else await wishlist.add($state.snapshot(book));
    } finally {
      wishBusy = false;
    }
  }
</script>

{#snippet sourceRows(title: string, list: Source[])}
  {#if list.length}
    <div class="group">
      <h3 class="eyebrow small">{title}</h3>
      {#each list as src (src.id)}
        <div class="row">
          <span class="letter">{src.name.charAt(0).toUpperCase()}</span>
          <div class="row-text">
            <div class="row-name">{src.name}</div>
            <div class="row-sub">
              {src.kind === 'custom' ? 'Your source · opens a search for this book' : 'Opens a search for this book in your browser'}
            </div>
          </div>
          <button class="btn sm" onclick={() => openSource(src)}>
            Open
            <Icon name="external" size={15} />
          </button>
        </div>
      {/each}
    </div>
  {/if}
{/snippet}

<section aria-labelledby="cat-h">
  <button class="btn back" onclick={() => nav.goBack(back)}>
    <Icon name="back" />
    {nav.backLabel(back)}
  </button>

  <div class="layout">
    <div class="cover-col">
      <Cover title={book.title} author={book.author} coverUrl={book.coverUrl} />
    </div>

    <div class="info">
      <div>
        {#if book.year}
          <div class="year mono">First published {book.year}</div>
        {/if}
        <h1 id="cat-h" class="title">{book.title}</h1>
        <p class="author">{book.author || 'Unknown author'}</p>
      </div>

      <div class="pills">
        <span class="pill">Not in your library</span>
        {#if book.isFree}
          <span class="pill teal">Public domain · free</span>
        {/if}
      </div>

      <div class="wish-row">
        {#if wish}
          <span class="on-wish">
            <Icon name="check" />
            On your wishlist. Shelf matches the file when it arrives.
          </span>
          <button class="btn sm ghost" onclick={toggleWish} disabled={wishBusy}>Remove</button>
        {:else}
          <button class="btn" onclick={toggleWish} disabled={wishBusy}>
            <Icon name="bookmark" />
            Add to wishlist
          </button>
        {/if}
      </div>

      <div class="get">
        <h2>Get this book</h2>

        {#if waiting}
          <div class="waiting" role="status">
            <span class="spinner" aria-hidden="true"></span>
            <div class="waiting-text">
              <strong>Waiting for your download from {waiting.source}…</strong>
              <span>
                {#if watching}
                  When the file lands in your Downloads folder, Shelf adds it and shows it here. Bought it from the Kobo Store?
                  It appears on your Kobo over Wi-Fi by itself.
                {:else}
                  Shelf isn't watching a folder, so it won't notice the download. Watch Downloads, or add the file yourself.
                {/if}
              </span>
              <div class="waiting-actions">
                {#if !watching}<button class="btn sm primary" onclick={watchDownloads}>Watch Downloads</button>{/if}
                <button class="btn sm" onclick={() => imports.pickAndImport()}>Add the file myself…</button>
                <button class="btn sm ghost" onclick={() => arrivals.stopWaiting(book)}>Stop waiting</button>
              </div>
            </div>
          </div>
        {:else if !watching && !book.isFree}
          <p class="note">
            Shelf isn't watching your Downloads folder, so after downloading you'll need to add the file yourself.
            <button class="link" onclick={watchDownloads}>Watch Downloads</button>
          </p>
        {/if}

        {#if book.isFree}
          <div class="group">
            <h3 class="eyebrow small">Free</h3>
            <div class="row">
              <span class="letter">{freeName.charAt(0)}</span>
              <div class="row-text">
                <div class="row-name">{freeName}</div>
                <div class="row-sub">Free EPUB · downloads straight into your library</div>
              </div>
              <button class="btn sm teal" onclick={addFree} disabled={adding}>
                {adding ? 'Adding…' : 'Add to library'}
              </button>
            </div>
          </div>
        {/if}

        {@render sourceRows('Buy', store)}
        {@render sourceRows('Your sources', custom)}

        {#if !hasSources}
          <p class="muted">
            No shops are switched on.
            <button class="link-btn" onclick={() => nav.go('sources')}>Turn some on in Settings</button>.
          </p>
        {/if}
        {#if custom.length === 0}
          <p class="small-note">
            Want another shop or library listed here?
            <button class="link-btn" onclick={() => nav.go('sources')}>Add it in Settings</button>
            and it shows on every book.
          </p>
        {/if}
      </div>
    </div>
  </div>
</section>

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
  .year {
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent);
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
  .pills,
  .wish-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
  }
  .on-wish {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent);
    font-weight: 500;
  }
  .get {
    max-width: 640px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding-top: 20px;
    border-top: 1px solid var(--border-soft);
  }
  .waiting {
    display: flex;
    gap: 14px;
    padding: 16px;
    border-radius: var(--radius-l);
    background: var(--accent-bg);
    border: 1px solid var(--accent-border);
  }
  .waiting-text {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .waiting-text span {
    font-size: 13px;
    color: var(--text-soft);
  }
  .waiting-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 4px;
  }
  .spinner {
    flex: none;
    width: 18px;
    height: 18px;
    margin-top: 2px;
    border-radius: 50%;
    border: 2px solid var(--accent-border);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation: none;
    }
  }
  .note {
    margin: 0;
    font-size: 13px;
    color: var(--muted);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    font-weight: 600;
    text-decoration: underline;
    cursor: pointer;
  }
  .get h2 {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .eyebrow.small {
    font-size: 12px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    border-radius: var(--r2);
    background: var(--surface-card);
    border: 1px solid var(--border-soft);
  }
  .letter {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 34px;
    height: 34px;
    border-radius: var(--radius-m);
    background: var(--surface-tile);
    font-weight: 700;
  }
  .row-text {
    flex: 1 1 220px;
    min-width: 0;
  }
  .row-name {
    font-weight: 600;
  }
  .row-sub {
    font-size: 13px;
    color: var(--muted);
  }
  .small-note {
    font-size: 13px;
    color: var(--muted);
  }
</style>
