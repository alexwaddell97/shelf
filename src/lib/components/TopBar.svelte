<script lang="ts">
  import { devices } from '$lib/stores/devices.svelte';
  import { imports } from '$lib/stores/imports.svelte';
  import { nav } from '$lib/stores/nav.svelte';
  import Icon from './Icon.svelte';

  const device = $derived(devices.connected[0]);
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

<header>
  <div class="search">
    <span class="search-icon"><Icon name="search" /></span>
    <label for="shelf-search" class="sr-only">Search books</label>
    <input
      id="shelf-search"
      type="search"
      bind:value={nav.query}
      placeholder="Search your library, or find new books"
      autocomplete="off"
      spellcheck="false"
      onkeydown={(e) => {
        if (e.key === 'Escape' && nav.query) {
          nav.query = '';
          e.stopPropagation();
        }
      }}
    />
  </div>

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
      disabled={syncing}
      title={queue > 0 ? `Send ${queue} waiting and update reading progress` : 'Update reading progress from your e-reader'}
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

  <button class="btn device-chip" onclick={() => nav.go('devices')}>
    <span class="dot" class:pulse={!!device} class:live={!!device}></span>
    <span class="device-name">{device ? device.name : 'No e-reader'}</span>
  </button>
</header>

<style>
  header {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 16px var(--pad-x);
    background: rgba(16, 17, 20, 0.92);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--border-faint);
  }
  .search {
    position: relative;
    flex: 1 1 320px;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 44px;
    padding: 0 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search-icon {
    display: flex;
    color: var(--faint);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--text);
    font-size: 15px;
  }
  input:focus-visible {
    outline: none;
  }
  .add {
    padding: 0 18px;
  }
  .device-chip {
    gap: 10px;
    padding: 0 14px;
    background: var(--surface);
    border-color: var(--border);
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
    background: var(--teal);
    border-color: var(--teal);
    color: #0b1a18;
    font-weight: 700;
    font-size: 15px;
  }
  .sync:hover:not(:disabled) {
    filter: brightness(1.08);
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
  .sync:disabled {
    cursor: default;
    opacity: 1;
  }
  /* Progress fills the button from the left while syncing. */
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    background: rgba(92, 200, 186, 0.22);
    transition: width 0.25s ease;
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
    background: #0b1a18;
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
