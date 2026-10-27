<script lang="ts">
  import type { ImportStep } from '$lib/api';
  import { plural } from '$lib/format';
  import { imports } from '$lib/stores/imports.svelte';
  import Icon from './Icon.svelte';

  const STEP_INDEX: Record<ImportStep, number> = { reading: 0, cover: 1, saving: 2, done: 3, failed: 3 };
  const STEP_LABEL: Record<ImportStep, string> = {
    reading: 'Reading metadata',
    cover: 'Extracting cover',
    saving: 'Saving to your library',
    done: 'Added to your library',
    failed: 'Could not import'
  };

  const total = $derived(imports.rows.length);
  const title = $derived(
    imports.active > 0
      ? 'Importing'
      : imports.failed > 0
        ? imports.finished > 0
          ? 'Imported with problems'
          : 'Import failed'
        : 'Imported'
  );
  const summary = $derived(
    imports.active > 0 ? `${imports.finished + imports.failed} of ${total}` : plural(total, 'file')
  );
</script>

{#if total > 0}
  <section class="panel-float" role="status" aria-label="Import progress">
    <div class="head">
      <span class="title">{title}</span>
      <span class="summary">{summary}</span>
      <button class="icon-btn bare close" aria-label="Close import panel" onclick={() => imports.dismiss()}>
        <Icon name="close" size={16} />
      </button>
    </div>

    <ul>
      {#each imports.rows as row (row.file)}
        {@const idx = STEP_INDEX[row.step]}
        <li class:failed={row.step === 'failed'}>
          <div class="line">
            <span class="status">
              {#if row.step === 'done'}
                <span class="check"><Icon name="check" size={11} /></span>
              {:else if row.step === 'failed'}
                <span class="cross"><Icon name="close" size={11} /></span>
              {:else}
                <span class="spin ring"></span>
              {/if}
            </span>
            <span class="name mono" title={row.file}>{row.name}</span>
          </div>
          <div class="stepper" aria-hidden="true">
            {#each [0, 1, 2, 3] as i (i)}
              <span class="seg" class:filled={row.step === 'failed' ? false : idx >= i} class:bad={row.step === 'failed'}></span>
            {/each}
          </div>
          <div class="step-label">{STEP_LABEL[row.step]}</div>
          {#if row.step === 'failed' && row.error}
            <div class="error">{row.error}</div>
          {/if}
          {#if row.matched && row.step !== 'failed'}
            <div class="matched">Matched your wishlist. The tile is now a real book.</div>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .panel-float {
    width: 320px;
    max-width: calc(100% - 32px);
    max-height: min(60vh, 480px);
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    border-radius: 22px;
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    border: 0;
    box-shadow: var(--glass-shadow);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title {
    font-weight: 700;
  }
  .summary {
    flex: 1;
    font-size: 11px;
    color: var(--faint);
  }
  .close {
    width: 28px;
    height: 28px;
    margin: -6px -6px -6px 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .name {
    min-width: 0;
    font-size: 12px;
    color: var(--text-soft);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status {
    flex: none;
    display: flex;
    width: 16px;
    height: 16px;
  }
  .check,
  .cross,
  .ring {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .check {
    background: var(--accent);
    color: var(--on-accent);
  }
  .cross {
    background: var(--danger);
    color: var(--on-accent);
  }
  .ring {
    border: 2px solid var(--border-dashed);
    border-top-color: var(--accent);
  }
  .stepper {
    display: flex;
    gap: 4px;
    padding-left: 26px;
  }
  .seg {
    flex: 1;
    height: 3px;
    border-radius: 999px;
    background: var(--track-strong);
  }
  .seg.filled {
    background: var(--accent);
  }
  .seg.bad {
    background: var(--danger);
  }
  .step-label {
    padding-left: 26px;
    font-size: 12px;
    color: var(--muted);
  }
  .failed .step-label {
    color: var(--danger);
  }
  .error {
    margin-left: 26px;
    font-size: 12px;
    color: var(--muted);
    word-break: break-word;
  }
  .matched {
    margin-left: 26px;
    padding: 8px 10px;
    border-radius: var(--radius-m);
    background: var(--accent-bg);
    color: var(--accent);
    font-size: 13px;
    font-weight: 600;
  }
</style>
