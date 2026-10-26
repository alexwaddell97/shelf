<script lang="ts">
  import { devices } from '$lib/stores/devices.svelte';
  import { imports } from '$lib/stores/imports.svelte';
  import { nav } from '$lib/stores/nav.svelte';
  import Icon from './Icon.svelte';
  import SearchBox from './SearchBox.svelte';
  import { tip } from '$lib/tooltip';

  const device = $derived(devices.connected[0]);

  // The bar sits flat on the page; once the page scrolls under it, it frosts over.
  let scrolled = $state(false);
  $effect(() => {
    const check = () => (scrolled = window.scrollY > 4);
    check();
    window.addEventListener('scroll', check, { passive: true });
    return () => window.removeEventListener('scroll', check);
  });
  const queue = $derived(devices.queueCount);
  const progress = $derived(devices.progress);
  const syncing = $derived(devices.syncing);
  const synced = $derived(!!device && devices.justSynced === device.id);
  // Only the copying stage has a known length; the bar never looks full before the end.
  const percent = $derived(
    progress?.phase === 'sending' && progress.total > 0 ? Math.round((progress.done / progress.total) * 90) : 0
  );
  const syncLabel = $derived(
    syncing
      ? progress?.phase === 'reading'
        ? 'Updating progress…'
        : progress?.phase === 'sending'
          ? devices.stageLabel
          : 'Syncing…'
      : synced
        ? 'Synced'
        : 'Sync'
  );

  let adding = $state(false);
  async function add() {
    if (adding) return;
    adding = true;
    try {
      await imports.pickAndImport();
    } finally {
      adding = false;
    }
  }
</script>

<header class:scrolled>
  <SearchBox />

  <button class="btn primary add" onclick={add} disabled={adding}>
    <Icon name="plus" />
    Add books
  </button>

  {#if device}
    <button
      class="btn sync"
      class:busy={syncing}
      class:done={synced && !syncing}
      onclick={() => devices.sync(device.id)}
      aria-disabled={syncing}
      use:tip={devices.syncTip(device)}
      aria-live="polite"
    >
      <span class="fill" style:width={syncing ? `${percent}%` : '0%'} aria-hidden="true"></span>
      <span class="sync-icon" class:spin={syncing}><Icon name={synced && !syncing ? 'check' : 'sync'} /></span>
      <span class="sync-label">{syncLabel}</span>
      {#if queue > 0 && !syncing}
        <span class="count">{queue}</span>
      {/if}
    </button>
  {/if}

  <button class="btn device-chip" onclick={() => nav.go('devices')} use:tip={devices.deviceTip(device)}>
    <span class="dot" class:pulse={!!device} class:live={!!device}></span>
    <span class="device-name">{device ? device.name : 'No e-reader'}</span>
  </button>
</header>

<style>
  /* A toolbar that sits flat on the page, like a Mac app's. When the page scrolls under
     it, it turns into a strip of frosted glass with a hairline beneath. */
  header {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: var(--float-gap) var(--pad-x) 12px;
    background: transparent;
    transition:
      background var(--dur) var(--ease),
      box-shadow var(--dur) var(--ease);
  }
  header.scrolled {
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    box-shadow: 0 0.5px 0 var(--border);
  }
  .add {
    padding: 0 18px;
  }
  .device-chip {
    gap: 10px;
    padding: 0 14px;
  }
  .device-name {
    font-weight: 500;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--dot-off);
  }
  .dot.live {
    background: var(--teal);
  }
  .sync {
    position: relative;
    overflow: hidden;
    gap: 10px;
    min-width: 132px;
    height: 44px;
    padding: 0 20px;
    justify-content: center;
    /* Tinted, not filled: one strong colour (Add books) is enough. */
    background: color-mix(in srgb, var(--teal) 12%, transparent);
    border-color: transparent;
    color: var(--teal);
    font-weight: 600;
  }
  .sync:hover:not(.busy) {
    background: color-mix(in srgb, var(--teal) 18%, transparent);
  }
  .sync.busy {
    background: var(--teal-bg);
    border-color: var(--teal-border);
    color: var(--teal);
  }
  .sync.done {
    background: var(--teal-bg);
    border-color: var(--teal);
    color: var(--teal);
  }
  .sync.busy {
    cursor: default;
    opacity: 1;
  }
  /* Progress fills the button from the left while syncing. */
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    background: var(--teal-wash);
    transition: width var(--dur) var(--ease);
  }
  .sync-icon,
  .sync-label,
  .count {
    position: relative;
    display: flex;
  }
  .count {
    min-width: 22px;
    justify-content: center;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--on-teal);
    color: var(--teal);
    font-size: 12px;
    font-weight: 700;
  }
  .spin {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
    .fill {
      transition: none;
    }
  }
</style>
