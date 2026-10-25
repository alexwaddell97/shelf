<script lang="ts">
  import { devices } from '$lib/stores/devices.svelte';
  import { imports } from '$lib/stores/imports.svelte';
  import { nav } from '$lib/stores/nav.svelte';
  import Icon from './Icon.svelte';

  const device = $derived(devices.connected[0]);
  const queue = $derived(devices.queueCount);

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

  <button class="btn device-chip" onclick={() => nav.go('devices')}>
    <span class="dot" class:pulse={!!device} class:live={!!device}></span>
    <span class="device-name">{device ? device.name : 'No e-reader'}</span>
    {#if device && queue > 0}
      <span class="waiting">{queue} waiting</span>
    {/if}
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
  .waiting {
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 700;
  }
</style>
