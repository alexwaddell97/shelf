<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { getBook, getReadingPosition, openBook, readBookFile, saveReadingPosition, type Book } from '$lib/api';
  import { errorMessage } from '$lib/format';
  import Icon from '$lib/components/Icon.svelte';
  import serifNormal from '@fontsource-variable/literata/files/literata-latin-opsz-normal.woff2?url';
  import serifItalic from '@fontsource-variable/literata/files/literata-latin-opsz-italic.woff2?url';
  import sansNormal from '@fontsource-variable/geist/files/geist-latin-wght-normal.woff2?url';

  // A book in its own window, rendered by foliate-js. Settings are kept per computer
  // (localStorage); where you are in the book is saved to the library.

  type ThemeId = 'paper' | 'sepia' | 'night';
  type FontId = 'book' | 'serif' | 'sans';
  type WidthId = 'narrow' | 'normal' | 'wide';
  interface Prefs {
    theme: ThemeId;
    fontSize: number;
    font: FontId;
    spacing: number;
    width: WidthId;
    columns: 1 | 2;
  }

  const THEMES: Record<ThemeId, { label: string; bg: string; fg: string; muted: string; link: string; chrome: string; border: string; dark: boolean }> = {
    paper: { label: 'Paper', bg: '#f8f4ec', fg: '#2b2620', muted: '#7a7166', link: '#8f5509', chrome: 'rgba(248,244,236,0.92)', border: '#e6ddcd', dark: false },
    sepia: { label: 'Sepia', bg: '#efe2c6', fg: '#4a3a28', muted: '#7d6a4f', link: '#7a4a10', chrome: 'rgba(239,226,198,0.92)', border: '#dccaa6', dark: false },
    night: { label: 'Night', bg: '#151517', fg: '#cfc9bd', muted: '#8f8a80', link: '#e5b25c', chrome: 'rgba(21,21,23,0.9)', border: '#2a2a2e', dark: true }
  };
  const WIDTHS: Record<WidthId, string> = { narrow: '560px', normal: '680px', wide: '820px' };
  const FONTS: Record<FontId, string> = {
    book: '',
    serif: "'Shelf Serif', Georgia, serif",
    sans: "'Shelf Sans', system-ui, sans-serif"
  };
  const PREFS_KEY = 'shelf.reader';
  const DEFAULT_PREFS: Prefs = { theme: 'paper', fontSize: 19, font: 'book', spacing: 1.55, width: 'normal', columns: 1 };

  function loadPrefs(): Prefs {
    try {
      return { ...DEFAULT_PREFS, ...JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') };
    } catch {
      return { ...DEFAULT_PREFS };
    }
  }

  const bookId = Number(page.params.id);
  let prefs = $state<Prefs>(loadPrefs());
  let book = $state<Book | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let stage = $state<HTMLDivElement | null>(null);
  // foliate-js's <foliate-view> element; untyped because the library ships no types.
  let view: any = null;
  let toc = $state<TocItem[]>([]);
  let fraction = $state(0);
  let chapter = $state('');
  let currentHref = $state('');
  let minutesLeft = $state<number | null>(null);
  let chromeVisible = $state(true);
  let tocOpen = $state(false);
  let settingsOpen = $state(false);
  let sliding = $state(false);

  interface TocItem {
    label: string;
    href: string;
    subitems?: TocItem[];
  }

  const theme = $derived(THEMES[prefs.theme]);
  const percent = $derived(Math.round(fraction * 100));

  // --- Styling the book's pages ---

  const fontFaces = () => {
    const abs = (url: string) => new URL(url, location.href).href;
    return `
      @font-face { font-family: 'Shelf Serif'; src: url('${abs(serifNormal)}') format('woff2'); font-weight: 200 900; font-style: normal; }
      @font-face { font-family: 'Shelf Serif'; src: url('${abs(serifItalic)}') format('woff2'); font-weight: 200 900; font-style: italic; }
      @font-face { font-family: 'Shelf Sans'; src: url('${abs(sansNormal)}') format('woff2'); font-weight: 100 900; font-style: normal; }`;
  };

  function bookCss(p: Prefs): string {
    const t = THEMES[p.theme];
    const family = FONTS[p.font];
    return `${fontFaces()}
      html { color-scheme: ${t.dark ? 'dark' : 'light'}; font-size: ${p.fontSize}px !important; }
      html, body { background: transparent !important; color: ${t.fg} !important; }
      body { font-size: 1rem !important; ${family ? `font-family: ${family} !important;` : ''} }
      ${family ? `p, li, blockquote, dd, div, span { font-family: inherit !important; }` : ''}
      body *:not(a) { color: inherit !important; }
      p, li, blockquote, dd {
        line-height: ${p.spacing} !important;
        text-align: justify;
        hyphens: auto; -webkit-hyphens: auto;
        widows: 2; orphans: 2;
      }
      [align="left"] { text-align: left; } [align="right"] { text-align: right; } [align="center"] { text-align: center; }
      a:link, a:visited { color: ${t.link} !important; }
      ::selection { background: ${t.dark ? 'rgba(229,178,92,0.35)' : 'rgba(143,85,9,0.22)'}; }
      pre { white-space: pre-wrap !important; }
      img, svg { max-width: 100%; height: auto; }
      ${t.dark ? 'img { opacity: 0.9; }' : ''}`;
  }

  function applyPrefs() {
    if (!view?.renderer) return;
    const r = view.renderer;
    r.setAttribute('flow', 'paginated');
    r.setAttribute('gap', '7%');
    r.setAttribute('margin', '56px');
    r.setAttribute('max-inline-size', WIDTHS[prefs.width]);
    r.setAttribute('max-column-count', String(prefs.columns));
    r.setStyles?.(bookCss(prefs));
  }

  function setPrefs(patch: Partial<Prefs>) {
    prefs = { ...prefs, ...patch };
    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
    } catch {
      /* storage unavailable */
    }
    applyPrefs();
    void syncWindowTheme();
  }

  async function syncWindowTheme() {
    try {
      const win = getCurrentWindow();
      await win.setTheme(theme.dark ? 'dark' : 'light');
      await win.setBackgroundColor(theme.bg);
    } catch {
      /* not available outside the app */
    }
  }

  // --- Saving your place ---

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let pending: { cfi: string; fraction: number } | null = null;

  function queueSave(cfi: string, frac: number) {
    pending = { cfi, fraction: frac };
    clearTimeout(saveTimer);
    saveTimer = setTimeout(flushSave, 1200);
  }

  function flushSave() {
    clearTimeout(saveTimer);
    if (!pending) return;
    const { cfi, fraction: f } = pending;
    pending = null;
    void saveReadingPosition(bookId, cfi, f).catch(() => {});
  }

  // --- Turning pages and showing the toolbar ---

  const next = () => void view?.next();
  const prev = () => void view?.prev();

  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  function poke() {
    chromeVisible = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      if (!tocOpen && !settingsOpen && !sliding) chromeVisible = false;
    }, 2600);
  }

  let lastWheel = 0;
  function onwheel(e: WheelEvent) {
    const now = Date.now();
    if (now - lastWheel < 280 || Math.abs(e.deltaY) < 4) return;
    lastWheel = now;
    if (e.deltaY > 0) next();
    else prev();
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (tocOpen || settingsOpen) {
        tocOpen = settingsOpen = false;
        e.preventDefault();
      }
      return;
    }
    if ((e.target as HTMLElement | null)?.closest?.('input, select, textarea')) return;
    if (e.key === ' ' && (e.target as HTMLElement | null)?.closest?.('button')) return;
    if (e.key === 'ArrowRight' || e.key === 'PageDown' || (e.key === ' ' && !e.shiftKey)) {
      e.preventDefault();
      if (e.key === 'ArrowRight') void view?.goRight();
      else next();
    } else if (e.key === 'ArrowLeft' || e.key === 'PageUp' || (e.key === ' ' && e.shiftKey)) {
      e.preventDefault();
      if (e.key === 'ArrowLeft') void view?.goLeft();
      else prev();
    } else if (e.key === 'Home') {
      void view?.goToFraction(0);
    } else if (e.key === 'End') {
      void view?.goToFraction(1);
    }
  }

  // Each chapter is its own document inside a frame, so it needs its own listeners.
  function onSectionLoad({ detail }: CustomEvent<{ doc: Document }>) {
    const doc = detail.doc;
    doc.addEventListener('keydown', onkey);
    doc.addEventListener('wheel', onwheel, { passive: true });
    doc.addEventListener('mousemove', poke);
    doc.addEventListener('click', (e: MouseEvent) => {
      if ((e.target as Element | null)?.closest?.('a')) return;
      if (doc.getSelection()?.toString()) return;
      if (tocOpen || settingsOpen) {
        tocOpen = settingsOpen = false;
        return;
      }
      chromeVisible = !chromeVisible;
      clearTimeout(hideTimer);
    });
  }

  function onRelocate({ detail }: CustomEvent<any>) {
    if (typeof detail.fraction === 'number') fraction = detail.fraction;
    chapter = detail.tocItem?.label?.trim() ?? '';
    currentHref = detail.tocItem?.href ?? '';
    const left = detail.time?.section;
    minutesLeft = typeof left === 'number' ? Math.max(0, Math.round(left)) : null;
    if (detail.cfi) queueSave(detail.cfi, detail.fraction ?? fraction);
  }

  function goToc(item: TocItem) {
    tocOpen = false;
    void view?.goTo(item.href);
  }

  // --- Opening the book ---

  onMount(() => {
    void syncWindowTheme();
    window.addEventListener('keydown', onkey);
    window.addEventListener('beforeunload', flushSave);
    void (async () => {
      try {
        book = await getBook(bookId);
        const bytes = await readBookFile(bookId);
        const name = book.filePath.split(/[\\/]/).pop() ?? 'book.epub';
        const file = new File([bytes], name);

        await import('foliate-js/view.js');
        view = document.createElement('foliate-view');
        stage?.prepend(view);
        view.addEventListener('load', onSectionLoad);
        view.addEventListener('relocate', onRelocate);
        await view.open(file);
        applyPrefs();
        toc = (view.book?.toc ?? []) as TocItem[];
        const saved = await getReadingPosition(bookId).catch(() => null);
        await view.init({ lastLocation: saved?.position ?? null, showTextStart: !saved });
        loading = false;
        poke();
      } catch (e) {
        error = errorMessage(e);
        loading = false;
      }
    })();
    return () => {
      flushSave();
      window.removeEventListener('keydown', onkey);
      window.removeEventListener('beforeunload', flushSave);
    };
  });
</script>

<svelte:head><title>{book ? `${book.title} — Shelf` : 'Shelf'}</title></svelte:head>

{#snippet tocList(items: TocItem[], depth: number)}
  <ul style:--depth={depth}>
    {#each items as item, i (item.href + i)}
      <li>
        <button class:current={item.href === currentHref} onclick={() => goToc(item)}>{item.label}</button>
        {#if item.subitems?.length}{@render tocList(item.subitems, depth + 1)}{/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div
  class="reader"
  class:dark={theme.dark}
  class:hide-chrome={!chromeVisible && !loading}
  style:--bg={theme.bg}
  style:--fg={theme.fg}
  style:--muted={theme.muted}
  style:--link={theme.link}
  style:--chrome={theme.chrome}
  style:--line={theme.border}
  role="presentation"
  onmousemove={poke}
>
  <header class="bar top">
    <button class="icon" aria-label="Contents" aria-expanded={tocOpen} onclick={() => ((tocOpen = !tocOpen), (settingsOpen = false))} disabled={!toc.length}>
      <Icon name="list" size={20} />
    </button>
    <div class="titles">
      <span class="book-title">{book?.title ?? ''}</span>
      {#if chapter}<span class="chapter">{chapter}</span>{/if}
    </div>
    <button class="icon type" aria-label="Reading settings" aria-expanded={settingsOpen} onclick={() => ((settingsOpen = !settingsOpen), (tocOpen = false))}>
      Aa
    </button>
  </header>

  <div class="stage" bind:this={stage} {onwheel}>
    <button class="edge left" aria-label="Previous page" onclick={prev}></button>
    <button class="edge right" aria-label="Next page" onclick={next}></button>
  </div>

  <footer class="bar bottom">
    <span class="where">{chapter}{#if minutesLeft !== null && minutesLeft > 0}<span class="sep">·</span>{minutesLeft < 1 ? 'under a minute' : `about ${minutesLeft} min`} left in chapter{/if}</span>
    <input
      class="slider"
      type="range"
      min="0"
      max="1"
      step="0.001"
      value={fraction}
      aria-label="Position in book"
      onpointerdown={() => (sliding = true)}
      onpointerup={() => ((sliding = false), poke())}
      onchange={(e) => view?.goToFraction(Number(e.currentTarget.value))}
    />
    <span class="percent">{percent}%</span>
  </footer>

  {#if tocOpen}
    <aside class="drawer" aria-label="Contents">
      <div class="drawer-head">
        <h2>Contents</h2>
        <button class="icon" aria-label="Close contents" onclick={() => (tocOpen = false)}><Icon name="close" size={18} /></button>
      </div>
      <nav>{@render tocList(toc, 0)}</nav>
    </aside>
  {/if}

  {#if settingsOpen}
    <div class="settings" role="dialog" aria-label="Reading settings">
      <div class="row">
        <span class="label">Page</span>
        <div class="seg">
          {#each Object.entries(THEMES) as [id, t] (id)}
            <button
              class="swatch"
              class:on={prefs.theme === id}
              style:--swatch-bg={t.bg}
              style:--swatch-fg={t.fg}
              aria-pressed={prefs.theme === id}
              onclick={() => setPrefs({ theme: id as ThemeId })}>{t.label}</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <span class="label">Text size</span>
        <div class="stepper">
          <button aria-label="Smaller text" onclick={() => setPrefs({ fontSize: Math.max(13, prefs.fontSize - 1) })} disabled={prefs.fontSize <= 13}>A−</button>
          <span class="value">{prefs.fontSize}</span>
          <button aria-label="Larger text" onclick={() => setPrefs({ fontSize: Math.min(32, prefs.fontSize + 1) })} disabled={prefs.fontSize >= 32}>A+</button>
        </div>
      </div>
      <div class="row">
        <span class="label">Font</span>
        <div class="seg">
          <button class:on={prefs.font === 'book'} aria-pressed={prefs.font === 'book'} onclick={() => setPrefs({ font: 'book' })}>Book’s own</button>
          <button class="serif" class:on={prefs.font === 'serif'} aria-pressed={prefs.font === 'serif'} onclick={() => setPrefs({ font: 'serif' })}>Literata</button>
          <button class="sans" class:on={prefs.font === 'sans'} aria-pressed={prefs.font === 'sans'} onclick={() => setPrefs({ font: 'sans' })}>Sans</button>
        </div>
      </div>
      <div class="row">
        <span class="label">Line spacing</span>
        <div class="seg">
          {#each [[1.35, 'Tight'], [1.55, 'Normal'], [1.8, 'Loose']] as [value, label] (value)}
            <button class:on={prefs.spacing === value} aria-pressed={prefs.spacing === value} onclick={() => setPrefs({ spacing: value as number })}>{label}</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <span class="label">Margins</span>
        <div class="seg">
          {#each [['wide', 'Narrow'], ['normal', 'Normal'], ['narrow', 'Wide']] as [value, label] (value)}
            <button class:on={prefs.width === value} aria-pressed={prefs.width === value} onclick={() => setPrefs({ width: value as WidthId })}>{label}</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <span class="label">Layout</span>
        <div class="seg">
          <button class:on={prefs.columns === 1} aria-pressed={prefs.columns === 1} onclick={() => setPrefs({ columns: 1 })}>One page</button>
          <button class:on={prefs.columns === 2} aria-pressed={prefs.columns === 2} onclick={() => setPrefs({ columns: 2 })}>Two pages</button>
        </div>
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="center">
      <div class="spinner" aria-hidden="true"></div>
      <p>{book ? `Opening ${book.title}…` : 'Opening…'}</p>
    </div>
  {:else if error}
    <div class="center">
      <h2>This book couldn’t be opened</h2>
      <p class="muted">{error}</p>
      {#if book}<button class="plain" onclick={() => openBook(bookId)}>Open in another app</button>{/if}
    </div>
  {/if}
</div>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
  }
  .reader {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--fg);
    font-family: 'Geist Variable', system-ui, sans-serif;
    transition:
      background 0.25s ease,
      color 0.25s ease;
  }

  /* Bars float over the page and fade out while reading. */
  .bar {
    position: absolute;
    left: 0;
    right: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 16px;
    background: var(--chrome);
    backdrop-filter: blur(10px);
    transition:
      opacity 0.3s ease,
      transform 0.3s ease;
  }
  .top {
    top: 0;
    border-bottom: 1px solid var(--line);
  }
  .bottom {
    bottom: 0;
    border-top: 1px solid var(--line);
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .hide-chrome .top {
    opacity: 0;
    transform: translateY(-8px);
    pointer-events: none;
  }
  .hide-chrome .bottom {
    opacity: 0;
    pointer-events: none;
  }
  .titles {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    line-height: 1.25;
  }
  .book-title {
    font-family: 'Literata Variable', Georgia, serif;
    font-weight: 600;
    font-size: 14px;
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chapter {
    font-size: 12px;
    color: var(--muted);
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  button {
    font: inherit;
    color: inherit;
  }
  .icon {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: 9px;
    background: transparent;
    cursor: pointer;
  }
  .icon:hover:not(:disabled) {
    background: color-mix(in srgb, var(--fg) 8%, transparent);
  }
  .icon:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .type {
    font-family: 'Literata Variable', Georgia, serif;
    font-size: 17px;
    font-weight: 600;
  }
  button:focus-visible,
  .slider:focus-visible {
    outline: 2px solid var(--link);
    outline-offset: 2px;
  }

  .stage {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .stage :global(foliate-view) {
    position: absolute;
    inset: 0;
    display: block;
  }
  /* Page-turn strips along the left and right edges. */
  .edge {
    position: absolute;
    top: 56px;
    bottom: 56px;
    z-index: 2;
    width: 9%;
    min-width: 48px;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  .edge.left {
    left: 0;
  }
  .edge.right {
    right: 0;
  }
  .edge:hover {
    background: linear-gradient(90deg, color-mix(in srgb, var(--fg) 5%, transparent), transparent);
  }
  .edge.right:hover {
    background: linear-gradient(270deg, color-mix(in srgb, var(--fg) 5%, transparent), transparent);
  }

  .where {
    flex: 1 1 0;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sep {
    margin: 0 6px;
  }
  .slider {
    flex: 0 1 280px;
    accent-color: var(--link);
    cursor: pointer;
  }
  .percent {
    min-width: 36px;
    text-align: right;
  }

  .drawer {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    z-index: 8;
    width: min(360px, 86vw);
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-right: 1px solid var(--line);
    box-shadow: 12px 0 40px rgba(0, 0, 0, 0.18);
    animation: slide-in 0.2s ease-out;
  }
  @keyframes slide-in {
    from {
      transform: translateX(-24px);
      opacity: 0;
    }
  }
  .drawer-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 14px 10px 22px;
    border-bottom: 1px solid var(--line);
  }
  .drawer h2 {
    margin: 0;
    font-family: 'Literata Variable', Georgia, serif;
    font-size: 19px;
    font-weight: 600;
  }
  .drawer nav {
    flex: 1;
    overflow-y: auto;
    padding: 8px 8px 24px;
  }
  .drawer ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .drawer li button {
    display: block;
    width: 100%;
    padding: 8px 12px 8px calc(14px + var(--depth) * 16px);
    border: 0;
    border-radius: 8px;
    background: transparent;
    text-align: left;
    font-size: 14px;
    line-height: 1.35;
    cursor: pointer;
  }
  .drawer li button:hover {
    background: color-mix(in srgb, var(--fg) 7%, transparent);
  }
  .drawer li button.current {
    color: var(--link);
    font-weight: 600;
    background: color-mix(in srgb, var(--link) 10%, transparent);
  }

  .settings {
    position: absolute;
    top: 60px;
    right: 14px;
    z-index: 8;
    width: min(360px, calc(100vw - 28px));
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px;
    border-radius: 16px;
    background: var(--bg);
    border: 1px solid var(--line);
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.22);
    animation: pop 0.18s ease-out;
  }
  @keyframes pop {
    from {
      transform: translateY(-6px) scale(0.98);
      opacity: 0;
    }
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .seg {
    display: flex;
    gap: 6px;
  }
  .seg button,
  .stepper button {
    flex: 1;
    height: 36px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: transparent;
    font-size: 13px;
    cursor: pointer;
  }
  .seg button.on {
    border-color: var(--link);
    color: var(--link);
    font-weight: 600;
    background: color-mix(in srgb, var(--link) 8%, transparent);
  }
  .seg .serif {
    font-family: 'Literata Variable', Georgia, serif;
  }
  .swatch {
    background: var(--swatch-bg) !important;
    color: var(--swatch-fg) !important;
  }
  .swatch.on {
    box-shadow: 0 0 0 2px var(--link);
  }
  .stepper {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .stepper button {
    font-family: 'Literata Variable', Georgia, serif;
    font-size: 15px;
  }
  .stepper button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .stepper .value {
    min-width: 32px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .center {
    position: absolute;
    inset: 0;
    z-index: 3;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    text-align: center;
    background: var(--bg);
  }
  .center h2 {
    margin: 0;
    font-family: 'Literata Variable', Georgia, serif;
    font-size: 22px;
  }
  .center p {
    margin: 0;
    max-width: 46ch;
  }
  .muted {
    color: var(--muted);
  }
  .plain {
    margin-top: 6px;
    padding: 9px 16px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: transparent;
    cursor: pointer;
  }
  .spinner {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2.5px solid color-mix(in srgb, var(--fg) 15%, transparent);
    border-top-color: var(--link);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bar,
    .reader {
      transition: none;
    }
    .drawer,
    .settings,
    .spinner {
      animation: none;
    }
  }
</style>
