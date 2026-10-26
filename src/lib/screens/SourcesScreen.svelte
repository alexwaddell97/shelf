<script lang="ts">
  import { openDefaultAppsSettings, type Source } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import Toggle from '$lib/components/Toggle.svelte';
  import { onMount } from 'svelte';
  import { basename } from '$lib/format';
  import { calibre } from '$lib/stores/calibre.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { theme, type ThemePref } from '$lib/stores/theme.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';

  const THEMES: { id: ThemePref; label: string; icon: 'monitor' | 'sun' | 'moon' }[] = [
    { id: 'light', label: 'Light', icon: 'sun' },
    { id: 'dark', label: 'Dark', icon: 'moon' },
    { id: 'system', label: 'System', icon: 'monitor' }
  ];

  const KIND_LABEL: Record<Source['kind'], string> = { store: 'Store', free: 'Free', custom: 'Custom' };

  let newName = $state('');
  let newUrl = $state('');
  let problem = $state<string | null>(null);
  let saving = $state(false);

  onMount(() => {
    void calibre.detect(true);
  });

  const os = /Windows/i.test(navigator.userAgent) ? 'windows' : /Mac/i.test(navigator.userAgent) ? 'mac' : 'linux';
  let showHelp = $state(false);

  async function chooseDefaultApp() {
    const opened = await attempt('Could not open Default apps settings', openDefaultAppsSettings);
    // false means this system has no such settings page: show the manual steps instead
    if (opened === false) showHelp = true;
  }

  const folders = $derived(settings.value?.watchFolders ?? []);

  function toggle(src: Source, enabled: boolean) {
    void settings.saveSources(settings.sources.map((s) => (s.id === src.id ? { ...s, enabled } : s)));
  }

  async function removeSource(src: Source) {
    const ok = await settings.saveSources(settings.sources.filter((s) => s.id !== src.id));
    if (ok) toasts.success(`Removed ${src.name}.`);
  }

  function validate(name: string, url: string): string | null {
    if (!name) return 'Give the source a name.';
    if (!url) return 'Add a search URL.';
    if (!url.includes('{query}')) return 'Put {query} in the URL where the search text should go.';
    try {
      const u = new URL(url.replace('{query}', 'test'));
      if (u.protocol !== 'http:' && u.protocol !== 'https:') return 'The URL should start with http:// or https://.';
    } catch {
      return 'That does not look like a web address. Try something like https://example.com/search?q={query}.';
    }
    if (settings.sources.some((s) => s.name.toLowerCase() === name.toLowerCase())) {
      return `You already have a source called ${name}.`;
    }
    return null;
  }

  async function addSource(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    const name = newName.trim();
    const url = newUrl.trim();
    problem = validate(name, url);
    if (problem) return;
    saving = true;
    try {
      const src: Source = {
        id: `custom-${Date.now().toString(36)}`,
        name,
        kind: 'custom',
        urlTemplate: url,
        enabled: true
      };
      const ok = await settings.saveSources([...settings.sources, src]);
      if (ok) {
        newName = '';
        newUrl = '';
        toasts.success(`Added ${name}. It now shows in searches and on every book.`);
      }
    } finally {
      saving = false;
    }
  }
</script>

<section aria-labelledby="src-h">
  <div>
    <h1 id="src-h" class="page-title">Settings</h1>
    <p class="page-sub">How Shelf looks, where it finds books, and the sites it can search for you.</p>
    <button class="link-btn rerun" onclick={() => (settings.rerunSetup = true)}>Run first-time setup again</button>
  </div>

  <div class="panel appearance">
    <div class="appearance-text">
      <h2>Appearance</h2>
      <p class="small">
        {theme.pref === 'system'
          ? `Following your system: Shelf is using the ${theme.resolved} theme right now.`
          : `Shelf always uses the ${theme.pref} theme.`}
      </p>
    </div>
    <div class="segmented" role="radiogroup" aria-label="Theme">
      {#each THEMES as t (t.id)}
        <button
          class="seg"
          class:on={theme.pref === t.id}
          role="radio"
          aria-checked={theme.pref === t.id}
          onclick={() => theme.set(t.id)}
        >
          <Icon name={t.icon} size={16} />
          {t.label}
        </button>
      {/each}
    </div>
  </div>

  <div class="group-head">
    <h2 class="serif">Search sources</h2>
    <p class="small">
      Search links shown when you look for a book. Each one opens the site in your browser with your search filled in.
      Shelf never downloads from them itself.
    </p>
  </div>

  <div class="list">
    {#each settings.sources as src (src.id)}
      <div class="source">
        <span class="letter">{src.name.charAt(0).toUpperCase()}</span>
        <div class="source-text">
          <div class="source-name">
            <span>{src.name}</span>
            <span class="kind">{KIND_LABEL[src.kind]}</span>
          </div>
          <div class="url mono" title={src.urlTemplate}>{src.urlTemplate}</div>
        </div>
        {#if src.kind === 'custom'}
          <button class="icon-btn bare" aria-label="Remove {src.name}" title="Remove this source" onclick={() => removeSource(src)}>
            <Icon name="close" size={16} />
          </button>
        {/if}
        <Toggle pressed={src.enabled} label="Show {src.name} in searches" onchange={(v) => toggle(src, v)} />
      </div>
    {:else}
      <p class="none">{settings.sourcesLoaded ? 'No sources yet. Add one below.' : 'Loading your sources…'}</p>
    {/each}

    <form class="add" onsubmit={addSource} aria-label="Add a custom source">
      <div class="field name">
        <label for="src-name">Name</label>
        <input id="src-name" class="input" bind:value={newName} placeholder="My bookshop" autocomplete="off" />
      </div>
      <div class="field url">
        <label for="src-url">Search URL, with {'{query}'} where the search text goes</label>
        <input
          id="src-url"
          class="input mono"
          bind:value={newUrl}
          placeholder="https://example.com/search?q={'{query}'}"
          autocomplete="off"
          spellcheck="false"
        />
      </div>
      <button type="submit" class="btn primary sm" disabled={saving}>Add source</button>
      {#if problem}
        <p class="problem" role="alert">{problem}</p>
      {/if}
    </form>
  </div>

  <div class="cards">
    <div class="panel card">
      <h2>Watch folders</h2>
      <p class="small">New .epub, .kepub, .mobi, .azw3 and .pdf files in these folders are imported automatically.</p>
      {#if folders.length === 0}
        <p class="small faint">No folders are being watched.</p>
      {/if}
      {#each folders as f (f)}
        <div class="folder">
          <span class="pulse dot"></span>
          <span class="folder-path mono" title={f}>{f}</span>
          <button class="icon-btn bare" aria-label="Stop watching {basename(f)}" title="Stop watching" onclick={() => settings.removeWatchFolder(f)}>
            <Icon name="close" size={16} />
          </button>
        </div>
      {/each}
      <button class="btn sm add-folder" onclick={() => settings.addWatchFolder()} disabled={!settings.value}>
        <Icon name="plus" size={16} />
        Add folder
      </button>
    </div>

    <div class="panel card">
      <h2>Import from calibre</h2>
      <p class="small">
        Copies your calibre books, covers, series and tags into Shelf. Your calibre library is only read, never changed.
      </p>
      {#if calibre.running}
        <p class="small status" role="status">
          Importing{#if calibre.progress && calibre.progress.total > 0}&nbsp;{calibre.progress.done} of {calibre.progress.total}{/if}…
        </p>
      {:else if !calibre.detected}
        <p class="small faint">Looking for your calibre library…</p>
      {:else if calibre.found}
        <div class="folder static">
          <Icon name="library" size={16} />
          <span class="folder-path mono" title={calibre.found}>{calibre.found}</span>
        </div>
      {:else}
        <p class="small faint">No calibre library found on this PC.</p>
      {/if}
      <div class="calibre-btns">
        {#if calibre.found}
          <button
            class="btn sm primary"
            disabled={calibre.running || calibre.inspecting}
            onclick={() => calibre.found && calibre.prepare(calibre.found)}
          >
            {calibre.inspecting ? 'Checking…' : 'Import'}
          </button>
        {/if}
        <button class="btn sm" disabled={calibre.running || calibre.inspecting} onclick={() => calibre.choose()}>
          {calibre.found ? 'Choose a different library…' : 'Choose a calibre library…'}
        </button>
      </div>
    </div>

    <div class="panel card">
      <h2>Open ebooks with Shelf</h2>
      <p class="small">
        Once Shelf is the app for .epub, .mobi and .azw3 files, double-clicking one in your file manager opens it here and adds it
        to your library. This works in the installed version of Shelf.
      </p>
      {#if os === 'windows'}
        <p class="small faint">In Default apps, search for Shelf and set it for .epub.</p>
      {/if}
      <div class="calibre-btns">
        <button class="btn sm" onclick={chooseDefaultApp}>Choose Shelf as the default app</button>
      </div>
      {#if showHelp || os !== 'windows'}
        <div class="help" role="note">
          {#if os === 'mac'}
            <p>Right-click an .epub file in Finder, choose Get Info, then Open with, pick Shelf and press Change All.</p>
          {:else if os === 'linux'}
            <p>Right-click an .epub file, choose Properties, then Open With, pick Shelf and set it as the default.</p>
          {:else}
            <p>Windows: In Default apps, search for Shelf and set it for .epub.</p>
          {/if}
        </div>
      {/if}
    </div>

    <div class="panel card">
      <h2>Library folder</h2>
      <p class="small">Where Shelf keeps the books you import. Move or back it up like any other folder.</p>
      <div class="folder static">
        <Icon name="folder" size={16} />
        <span class="folder-path mono" title={settings.value?.libraryDir}>{settings.value?.libraryDir ?? '…'}</span>
      </div>
    </div>
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-width: 920px;
  }
  .appearance {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .appearance h2,
  .group-head h2 {
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.005em;
  }
  .group-head {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-width: 70ch;
    margin-bottom: -8px;
  }
  .segmented {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: var(--r2);
    background: var(--surface-sunken);
    border: 1px solid var(--border-soft);
  }
  .seg {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 14px;
    border: 0;
    border-radius: calc(var(--r2) - 3px);
    background: transparent;
    color: var(--muted);
    font-weight: 500;
    transition:
      background-color var(--dur) var(--ease),
      color var(--dur) var(--ease),
      box-shadow var(--dur) var(--ease);
  }
  .seg:hover {
    color: var(--text);
  }
  .seg.on {
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: var(--shadow-1);
  }
  .rerun {
    margin-top: 8px;
    font-size: 13px;
  }
  .list {
    border-radius: var(--radius-xl);
    background: var(--surface-card);
    border: 1px solid var(--border-soft);
    overflow: hidden;
  }
  .source {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-faint);
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
  .source-text {
    flex: 1;
    min-width: 0;
  }
  .source-name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .kind {
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--surface-chip);
    color: var(--muted);
    font-size: 11px;
    font-weight: 400;
  }
  .url {
    font-size: 12px;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .none {
    padding: 18px 16px;
    color: var(--muted);
  }
  .add {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px;
    padding: 16px;
    background: var(--surface);
  }
  .add .name {
    flex: 1 1 160px;
  }
  .add .url {
    flex: 3 1 320px;
  }
  .problem {
    flex: 1 1 100%;
    color: var(--danger);
    font-size: 13px;
  }
  .cards {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
  }
  .card {
    flex: 1 1 300px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .card h2 {
    font-size: 16px;
    font-weight: 600;
  }
  .small {
    font-size: 13px;
    color: var(--muted);
  }
  .faint {
    color: var(--faint);
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 6px 6px 12px;
    border-radius: var(--radius-m);
    background: var(--surface-sunken);
    font-size: 12px;
  }
  .folder.static {
    padding: 10px 12px;
    color: var(--muted);
  }
  .folder-path {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--teal);
  }
  .calibre-btns {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: auto;
  }
  .help {
    padding: 10px 12px;
    border-radius: var(--radius-m);
    background: var(--surface-sunken);
    font-size: 13px;
    color: var(--text-soft);
  }
  .status {
    color: var(--accent);
  }
  .add-folder {
    align-self: flex-start;
    margin-top: 4px;
  }
</style>
