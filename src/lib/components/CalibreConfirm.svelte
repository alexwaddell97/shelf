<script lang="ts">
  import { plural } from '$lib/format';
  import { calibre } from '$lib/stores/calibre.svelte';
  import Modal from './Modal.svelte';

  const summary = $derived(calibre.pending);
  const importable = $derived(summary ? summary.bookCount - summary.unsupported : 0);
</script>

{#if summary}
  <Modal labelledby="calibre-h" maxWidth="500px" onclose={() => calibre.cancel()}>
    <div class="body">
      {#if importable > 0}
        <h2 id="calibre-h">Import {plural(summary.bookCount, 'book')} from calibre?</h2>
      {:else}
        <h2 id="calibre-h">Nothing to import</h2>
      {/if}
      <p class="path mono" title={summary.path}>{summary.path}</p>
      {#if importable > 0}
        <ul>
          <li>Shelf only reads calibre’s library. calibre’s own files are never changed.</li>
          <li>Books are not sent to your e-reader automatically. Queue the ones you want from the book page.</li>
          {#if summary.unsupported > 0}
            <li class="warn">
              {plural(summary.unsupported, 'book')} with no usable format (EPUB, KEPUB, AZW3, MOBI or PDF) will be skipped.
            </li>
          {/if}
        </ul>
      {:else}
        <p class="muted">
          This library has {plural(summary.bookCount, 'book')}, none with a format Shelf can import.
        </p>
      {/if}
      <div class="actions">
        <button class="btn" data-autofocus={importable === 0 ? '' : undefined} onclick={() => calibre.cancel()}>
          {importable > 0 ? 'Cancel' : 'Close'}
        </button>
        {#if importable > 0}
          <button class="btn primary" data-autofocus onclick={() => calibre.confirm()}>
            Import {plural(importable, 'book')}
          </button>
        {/if}
      </div>
    </div>
  </Modal>
{/if}

<style>
  .body {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h2 {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .path {
    font-size: 12px;
    color: var(--faint);
    word-break: break-all;
  }
  ul {
    margin: 0;
    padding: 0 0 0 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--text-soft);
  }
  .warn {
    color: var(--accent);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
  }
</style>
