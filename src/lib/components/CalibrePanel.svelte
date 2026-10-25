<script lang="ts">
  import { clamp, plural } from '$lib/format';
  import { calibre } from '$lib/stores/calibre.svelte';
  import Icon from './Icon.svelte';

  const p = $derived(calibre.progress);
  const pct = $derived(p && p.total > 0 ? clamp(Math.round((p.done / p.total) * 100)) : 0);
  const report = $derived(calibre.report);
</script>

{#if calibre.running}
  <section class="card" role="status" aria-label="calibre import progress">
    <div class="head">
      <span class="title">Importing from calibre</span>
      {#if p && p.total > 0}<span class="count mono">{p.done} of {p.total}</span>{/if}
    </div>
    <div class="bar" class:indeterminate={!p || p.total === 0}><div style:width="{pct}%"></div></div>
    <div class="current" title={p?.current ?? ''}>{p?.current ?? 'Starting…'}</div>
  </section>
{:else if report}
  <section class="card" role="status" aria-label="calibre import result">
    <div class="head">
      <span class="title">calibre import finished</span>
      <button class="icon-btn bare close" aria-label="Close" onclick={() => calibre.dismissReport()}>
        <Icon name="close" size={16} />
      </button>
    </div>
    <div class="stats">
      <div><div class="num">{report.imported}</div><div class="cap">imported</div></div>
      <div><div class="num">{report.skippedDuplicates}</div><div class="cap">duplicates skipped</div></div>
      {#if report.failed.length}
        <div><div class="num bad">{report.failed.length}</div><div class="cap">failed</div></div>
      {/if}
    </div>
    {#if report.failed.length}
      <details>
        <summary>Show the {plural(report.failed.length, 'book')} that failed</summary>
        <ul class="mono">
          {#each report.failed as line, i (i)}
            <li>{line}</li>
          {/each}
        </ul>
      </details>
    {/if}
  </section>
{/if}

<style>
  .card {
    width: 320px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    border-radius: var(--radius-xl);
    background: var(--surface-raised);
    border: 1px solid var(--border-control);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.55);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .title {
    font-weight: 700;
  }
  .count {
    font-size: 12px;
    color: var(--muted);
  }
  .close {
    width: 28px;
    height: 28px;
    margin: -6px -6px -6px 0;
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--track-strong);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease;
  }
  .bar.indeterminate div {
    width: 35% !important;
    animation: slide 1.1s ease-in-out infinite alternate;
  }
  @keyframes slide {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(185%);
    }
  }
  .current {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 22px;
  }
  .num {
    font-size: 22px;
    font-weight: 700;
  }
  .num.bad {
    color: var(--danger);
  }
  .cap {
    font-size: 12px;
    color: var(--muted);
  }
  summary {
    cursor: pointer;
    font-size: 13px;
    color: var(--accent);
  }
  ul {
    margin: 8px 0 0;
    padding: 0 0 0 16px;
    max-height: 180px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 11px;
    color: var(--text-soft);
    word-break: break-word;
  }
</style>
