<script lang="ts">
  import { basename } from '$lib/format';
  import { devices } from '$lib/stores/devices.svelte';
  import { goodreads } from '$lib/stores/goodreads.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { nav, type BrowseKind, type Section } from '$lib/stores/nav.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { wishlist } from '$lib/stores/wishlist.svelte';
  import Icon, { type IconName } from './Icon.svelte';

  const items: { section: Section; label: string; icon: IconName }[] = [
    { section: 'library', label: 'Library', icon: 'library' },
    { section: 'wishlist', label: 'Wishlist', icon: 'bookmark' },
    { section: 'devices', label: 'Devices', icon: 'device' },
    { section: 'sources', label: 'Sources', icon: 'link' },
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
</script>

<nav aria-label="Main">
  <div class="brand">
    <svg width="26" height="26" viewBox="0 0 26 26" aria-hidden="true">
      <rect x="2" y="4" width="5" height="19" rx="1.5" fill="var(--accent)"></rect>
      <rect x="9" y="7" width="5" height="16" rx="1.5" fill="var(--text)"></rect>
      <rect x="16.5" y="3.5" width="5" height="19" rx="1.5" fill="var(--teal)" transform="rotate(10 19 13)"></rect>
    </svg>
    <span class="brand-name">Shelf</span>
  </div>

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
        {:else if item.section === 'accounts' && goodreads.account}
          <span class="conn-dot" role="img" aria-label="Goodreads connected" title="Goodreads connected"></span>
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
            Plug one in over USB
          {/if}
        </span>
      </span>
    </button>
  </div>
</nav>

<style>
  nav {
    position: sticky;
    top: 0;
    align-self: start;
    height: 100vh;
    overflow-y: auto;
    padding: 22px 14px;
    display: flex;
    flex-direction: column;
    gap: 24px;
    background: var(--bg-nav);
    border-right: 1px solid var(--border-faint);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
  }
  .brand-name {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .items {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 40px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-m);
    background: transparent;
    color: var(--muted);
    font-weight: 500;
    text-align: left;
    transition: box-shadow 0.15s ease;
  }
  .item:hover {
    box-shadow: inset 0 0 0 999px rgba(255, 255, 255, 0.05);
  }
  .item.active {
    background: var(--surface-hover);
    color: var(--text);
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
    border-radius: var(--radius-l);
    background: var(--surface-card);
    border: 1px solid var(--border-faint);
    color: var(--text);
    text-align: left;
    min-width: 0;
  }
  .card.device {
    flex-direction: row;
    align-items: center;
    gap: 10px;
    transition: box-shadow 0.15s ease;
  }
  .card.device:hover {
    box-shadow: inset 0 0 0 999px rgba(255, 255, 255, 0.05);
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

  /* Narrow windows: collapse to an icon rail. */
  @media (max-width: 900px) {
    nav {
      padding: 18px 8px;
      align-items: center;
    }
    .brand {
      padding: 0;
    }
    .brand-name,
    .group-label,
    .label,
    .count,
    .watch,
    .device-text {
      display: none;
    }
    .items,
    .bottom {
      width: 100%;
    }
    .item {
      position: relative;
      justify-content: center;
      padding: 0;
    }
    .conn-dot {
      position: absolute;
      top: 8px;
      right: 10px;
    }
    .queue-badge {
      position: absolute;
      top: 2px;
      right: 2px;
      min-width: 16px;
      padding: 0 4px;
      font-size: 10px;
    }
    .card.device {
      padding: 12px 0;
      justify-content: center;
      width: 100%;
    }
  }
</style>
