<script lang="ts">
  import { basename } from '$lib/format';
  import { devices } from '$lib/stores/devices.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { nav, type BrowseKind, type Section } from '$lib/stores/nav.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { theme } from '$lib/stores/theme.svelte';
  import { ui } from '$lib/stores/ui.svelte';
  import { updates } from '$lib/stores/updates.svelte';
  import { wishlist } from '$lib/stores/wishlist.svelte';
  import Icon, { type IconName } from './Icon.svelte';

  const items: { section: Section; label: string; icon: IconName }[] = [
    { section: 'library', label: 'Library', icon: 'library' },
    { section: 'wishlist', label: 'Wishlist', icon: 'bookmark' },
    { section: 'devices', label: 'Devices', icon: 'device' },
    { section: 'sources', label: 'Settings', icon: 'sliders' },
    { section: 'accounts', label: 'Accounts', icon: 'user' }
  ];

  const browseItems: { kind: BrowseKind; label: string; icon: IconName }[] = [
    { kind: 'authors', label: 'Authors', icon: 'user' },
    { kind: 'series', label: 'Series', icon: 'layers' },
    { kind: 'collections', label: 'Collections', icon: 'bookmark' },
    { kind: 'tags', label: 'Tags', icon: 'tag' }
  ];

  const counts = $derived<Partial<Record<Section, number>>>({
    library: library.books.length,
    wishlist: wishlist.items.length
  });

  const device = $derived(devices.connected[0]);
  const queue = $derived(devices.queueCount);
  const folders = $derived(settings.value?.watchFolders ?? []);

  // Ctrl+B (Cmd+B on a Mac) collapses the sidebar to icons and back, as in most apps
  // with a sidebar.
  function onkeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === 'b' && !ui.narrow) {
      e.preventDefault();
      ui.toggleSidebar();
    }
  }
</script>

<svelte:window {onkeydown} />

<nav aria-label="Main" class:rail={ui.rail}>
  <div class="items">
    {#each items as item (item.section)}
      {@const active = nav.section === item.section}
      <button
        class="item"
        class:active
        aria-current={active ? 'page' : undefined}
        aria-label={item.label}
        title={item.label}
        onclick={() => nav.go(item.section)}
      >
        <Icon name={item.icon} />
        <span class="label">{item.label}</span>
        {#if item.section === 'devices' && queue > 0}
          <span class="queue-badge" aria-label="{queue} waiting to send">{queue}</span>
        {:else if item.section === 'sources' && updates.available}
          <span class="conn-dot update-dot" role="img" aria-label="Update available" title="Shelf {updates.available.version} is available"></span>
        {:else if counts[item.section] !== undefined}
          <span class="count mono">{counts[item.section]}</span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="items browse">
    <div class="group-label">Browse</div>
    {#each browseItems as item (item.kind)}
      {@const active = nav.section === item.kind}
      <button
        class="item small"
        class:active
        aria-current={active ? 'page' : undefined}
        aria-label={item.label}
        title={item.label}
        onclick={() => nav.browse(item.kind)}
      >
        <Icon name={item.icon} size={16} />
        <span class="label">{item.label}</span>
      </button>
    {/each}
  </div>

  <div class="bottom">
    {#if folders.length > 0}
      <div class="card watch">
        <div class="card-title">
          <Icon name="folder" size={16} />
          Watching {basename(folders[0])}{folders.length > 1 ? ` +${folders.length - 1}` : ''}
        </div>
        <div class="card-sub mono" title={folders[0]}>{folders[0]}</div>
      </div>
    {/if}
    <button class="card device" onclick={() => nav.go('devices')} title={device ? device.name : 'Devices'}>
      <span class="dot" class:pulse={!!device} class:live={!!device}></span>
      <span class="device-text">
        <span class="card-title">{device ? device.name : 'No e-reader connected'}</span>
        <span class="card-sub">
          {#if device}
            {queue ? `Connected · ${queue} waiting` : 'Connected · up to date'}
          {:else}
            Plug it in, then tap Connect on your Kobo
          {/if}
        </span>
      </span>
    </button>
    <div class="footer">
      <button
        class="theme-toggle"
        onclick={() => theme.toggle()}
        aria-label={theme.resolved === 'dark' ? 'Switch to the light theme' : 'Switch to the dark theme'}
        title={theme.resolved === 'dark' ? 'Switch to the light theme' : 'Switch to the dark theme'}
      >
        <Icon name={theme.resolved === 'dark' ? 'sun' : 'moon'} size={16} />
        <span class="toggle-label">{theme.resolved === 'dark' ? 'Light theme' : 'Dark theme'}</span>
      </button>
      {#if !ui.narrow}
        <button
          class="collapse"
          onclick={() => ui.toggleSidebar()}
          aria-label={ui.collapsed ? 'Show the full sidebar' : 'Collapse the sidebar to icons'}
          aria-pressed={ui.collapsed}
          title={ui.collapsed ? 'Show the full sidebar (Ctrl+B)' : 'Collapse to icons (Ctrl+B)'}
        >
          <Icon name="sidebar" size={16} />
        </button>
      {/if}
    </div>
  </div>
</nav>

<style>
  /* A floating pane of frosted glass, inset from the window's edges. */
  nav {
    position: sticky;
    top: var(--float-gap);
    align-self: start;
    height: calc(100vh - 2 * var(--float-gap));
    margin: var(--float-gap) 0 var(--float-gap) var(--float-gap);
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 24px;
    border-radius: var(--r-float);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--glass-shadow);
    scrollbar-width: none;
  }
  .items {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 40px;
    padding: 0 12px;
    border: 0;
    border-radius: 12px;
    background: transparent;
    color: var(--muted);
    font-weight: 500;
    text-align: left;
    transition: box-shadow var(--dur) var(--ease);
  }
  .item:hover {
    box-shadow: inset 0 0 0 999px var(--hover-wash);
  }
  /* The current page sits on a raised pill of brighter glass. */
  .item.active {
    background: var(--glass-selected);
    box-shadow: var(--glass-shine), 0 1px 3px rgba(0, 0, 0, 0.08);
    color: var(--text);
  }
  .item.active :global(svg) {
    color: var(--accent);
  }
  .theme-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 36px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r2);
    background: transparent;
    color: var(--muted);
    font-size: 13px;
    transition:
      box-shadow var(--dur) var(--ease),
      color var(--dur) var(--ease);
  }
  .theme-toggle:hover {
    color: var(--text);
    box-shadow: inset 0 0 0 999px var(--hover-wash);
  }
  .item.small {
    min-height: 34px;
    font-size: 13px;
  }
  .group-label {
    padding: 0 12px 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--faint);
  }
  .label {
    flex: 1;
  }
  .count {
    font-size: 12px;
    color: var(--faint);
  }
  .conn-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--teal);
  }
  .queue-badge {
    min-width: 20px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 11px;
    font-weight: 700;
    text-align: center;
  }
  .bottom {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    border-radius: 16px;
    background: var(--glass-tint);
    border: 0;
    box-shadow: var(--glass-shine), 0 0 0 0.5px var(--border-faint);
    color: var(--text);
    text-align: left;
    min-width: 0;
  }
  .card.device {
    flex-direction: row;
    align-items: center;
    gap: 10px;
    transition: box-shadow var(--dur) var(--ease);
  }
  .card.device:hover {
    box-shadow:
      inset 0 0 0 999px var(--hover-wash),
      var(--glass-shine),
      0 0 0 0.5px var(--border-faint);
  }
  .card-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
    min-width: 0;
  }
  .card-sub {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .watch .card-sub {
    font-size: 11px;
    color: var(--faint);
  }
  .device-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .device-text .card-title {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--dot-off);
  }
  .dot.live {
    background: var(--teal);
  }

  .footer {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .footer .theme-toggle {
    flex: 1;
  }
  .collapse {
    display: grid;
    place-items: center;
    flex: none;
    width: 36px;
    height: 36px;
    border: 0;
    border-radius: var(--r2);
    background: transparent;
    color: var(--muted);
    transition:
      box-shadow var(--dur) var(--ease),
      color var(--dur) var(--ease);
  }
  .collapse:hover {
    color: var(--text);
    box-shadow: inset 0 0 0 999px var(--hover-wash);
  }
  .update-dot {
    background: var(--accent);
  }

  /* The icon rail: collapsed by choice, or a narrow window. */
  nav.rail {
    padding: 12px 8px;
    align-items: center;
  }
  .rail .toggle-label,
  .rail .group-label,
  .rail .label,
  .rail .count,
  .rail .watch,
  .rail .device-text {
    display: none;
  }
  .rail .items,
  .rail .bottom {
    width: 100%;
  }
  .rail .item {
    position: relative;
    justify-content: center;
    padding: 0;
  }
  .rail .conn-dot {
    position: absolute;
    top: 8px;
    right: 10px;
  }
  .rail .queue-badge {
    position: absolute;
    top: 2px;
    right: 2px;
    min-width: 16px;
    padding: 0 4px;
    font-size: 10px;
  }
  .rail .card.device {
    padding: 12px 0;
    justify-content: center;
    width: 100%;
  }
  .rail .footer {
    flex-direction: column;
  }
  .rail .footer .theme-toggle {
    justify-content: center;
    width: 100%;
    padding: 0;
  }
  .rail .collapse {
    width: 100%;
  }
</style>
