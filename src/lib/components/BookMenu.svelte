<script lang="ts">
  import { tick } from 'svelte';
  import type { Book } from '$lib/api';
  import { bookActions } from '$lib/stores/bookActions.svelte';
  import { devices } from '$lib/stores/devices.svelte';
  import Modal from './Modal.svelte';

  interface Item {
    label: string;
    run: () => void;
    destructive?: boolean;
    /** Draw a divider above this item. */
    divider?: boolean;
  }

  const menu = $derived(bookActions.menu);

  function itemsFor(book: Book, open: () => void): Item[] {
    const items: Item[] = [
      { label: 'Open', run: open },
      { label: 'Edit details…', run: () => bookActions.edit(book, open) }
    ];
    let first = true;
    for (const device of devices.connected) {
      const on = book.onDevices.includes(device.id);
      const queued = book.queuedFor.includes(device.id);
      const item: Item = on
        ? queued
          ? { label: `Cancel update on ${device.name}`, run: () => bookActions.cancelSend(book, device.id) }
          : { label: `Remove from ${device.name}`, run: () => bookActions.removeFromDevice(book, device) }
        : queued
          ? { label: `Cancel sending to ${device.name}`, run: () => bookActions.cancelSend(book, device.id) }
          : { label: `Send to ${device.name}`, run: () => bookActions.send(book, device) };
      items.push({ ...item, divider: first });
      first = false;
    }
    items.push(
      { label: 'Read', run: () => bookActions.read(book), divider: true },
      { label: 'Open in another app', run: () => bookActions.openElsewhere(book) },
      { label: 'Show file in folder', run: () => bookActions.reveal(book) },
      { label: 'Remove from library…', run: () => bookActions.askDelete(book), destructive: true, divider: true }
    );
    return items;
  }

  const items = $derived(menu ? itemsFor(menu.book, menu.open) : []);

  let el = $state<HTMLDivElement | null>(null);
  let pos = $state({ x: 0, y: 0 });

  // Keep the menu on screen, then focus its first item for keyboard use.
  $effect(() => {
    if (!menu) return;
    pos = { x: menu.x, y: menu.y };
    void tick().then(() => {
      if (!el) return;
      const r = el.getBoundingClientRect();
      pos = {
        x: Math.max(8, Math.min(menu.x, window.innerWidth - r.width - 8)),
        y: Math.max(8, Math.min(menu.y, window.innerHeight - r.height - 8))
      };
      el.querySelector<HTMLButtonElement>('button')?.focus();
    });
  });

  function choose(item: Item) {
    bookActions.closeMenu();
    item.run();
  }

  function onkeydown(e: KeyboardEvent) {
    if (!el) return;
    const buttons = [...el.querySelectorAll<HTMLButtonElement>('button')];
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === 'Escape') {
      e.preventDefault();
      bookActions.closeMenu();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const next = e.key === 'ArrowDown' ? (i + 1) % buttons.length : (i - 1 + buttons.length) % buttons.length;
      buttons[next]?.focus();
    } else if (e.key === 'Tab') {
      bookActions.closeMenu();
    }
  }
</script>

<svelte:window
  onresize={() => bookActions.closeMenu()}
  onblur={() => bookActions.closeMenu()}
  onwheel={() => bookActions.closeMenu()}
/>

{#if menu}
  <!-- Clicking anywhere outside closes the menu. -->
  <div
    class="backdrop"
    role="presentation"
    onpointerdown={() => bookActions.closeMenu()}
    oncontextmenu={(e) => {
      e.preventDefault();
      bookActions.closeMenu();
    }}
  ></div>
  <div
    bind:this={el}
    class="menu"
    role="menu"
    tabindex="-1"
    aria-label="Actions for {menu.book.title}"
    style:left="{pos.x}px"
    style:top="{pos.y}px"
    {onkeydown}
  >
    <div class="heading" aria-hidden="true">{menu.book.title}</div>
    {#each items as item (item.label)}
      {#if item.divider}<div class="divider" role="separator"></div>{/if}
      <button role="menuitem" class:destructive={item.destructive} onclick={() => choose(item)}>{item.label}</button>
    {/each}
  </div>
{/if}

{#if bookActions.deleting}
  {@const book = bookActions.deleting}
  <Modal labelledby="menu-delete-h" maxWidth="460px" onclose={() => bookActions.cancelDelete()}>
    <div class="dialog-body">
      <h2 id="menu-delete-h">Remove “{book.title}” from your library?</h2>
      <p class="muted">
        It leaves your library and any send queue. Copies already on an e-reader stay there until you remove them.
      </p>
      <label class="check">
        <input type="checkbox" bind:checked={bookActions.deleteFile} />
        <span>Also delete the file from this PC</span>
      </label>
      <div class="dialog-actions">
        <button class="btn" data-autofocus onclick={() => bookActions.cancelDelete()} disabled={bookActions.busy}>
          Cancel
        </button>
        <button class="btn destructive" onclick={() => bookActions.confirmDelete()} disabled={bookActions.busy}>
          {bookActions.busy ? 'Removing…' : bookActions.deleteFile ? 'Remove and delete file' : 'Remove from library'}
        </button>
      </div>
    </div>
  </Modal>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
  }
  .menu {
    position: fixed;
    z-index: 61;
    min-width: 220px;
    max-width: 320px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    border: 0;
    border-radius: 16px;
    box-shadow: var(--glass-shadow);
  }
  .heading {
    padding: 6px 10px 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .divider {
    height: 1px;
    margin: 5px 4px;
    background: var(--border);
  }
  button {
    display: flex;
    align-items: center;
    min-height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }
  button:hover,
  button:focus-visible {
    background: var(--menu-hover);
    outline: none;
  }
  .destructive {
    color: var(--danger);
  }
  .dialog-body {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .dialog-body h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .dialog-body p {
    margin: 0;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 6px;
  }
</style>
