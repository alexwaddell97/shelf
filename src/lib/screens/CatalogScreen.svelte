<script lang="ts">
  import { downloadFree, openUrl, sourceUrl, type CatalogBook, type Source } from '$lib/api';
  import Cover from '$lib/components/Cover.svelte';
  import Icon from '$lib/components/Icon.svelte';
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
    if (ok) toasts.info(`Opened ${src.name} in your browser`, url);
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
            <button class="link-btn" onclick={() => nav.go('sources')}>Turn some on in Sources</button>.
          </p>
        {/if}
        {#if custom.length === 0}
          <p class="small-note">
            Want another shop or library listed here?
            <button class="link-btn" onclick={() => nav.go('sources')}>Add it in Sources</button>
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
    font-size: 42px;
    line-height: 1.05;
    font-weight: 700;
    letter-spacing: -0.03em;
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
    border-radius: 12px;
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
