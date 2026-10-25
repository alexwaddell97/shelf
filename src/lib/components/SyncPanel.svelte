<script lang="ts">
  import { clamp, plural } from '$lib/format';
  import { devices } from '$lib/stores/devices.svelte';
  import Icon from './Icon.svelte';

  // Always on screen (bottom right) while a sync runs, then a short summary of what happened.
  const p = $derived(devices.progress);
  const report = $derived(devices.lastReport);
  const name = $derived(devices.name(p?.deviceId ?? report?.deviceId ?? ''));
  const sending = $derived(p?.phase === 'sending' && p.total > 0);
  // The same 90% cap as the button: the last stretch is updating reading progress.
  const pct = $derived(sending && p ? clamp(Math.round((p.done / p.total) * 90)) : 0);
  const step = $derived(devices.stageLabel);
  const detail = $derived(sending && p?.current ? `“${p.current}”` : p?.phase === 'reading' ? 'Almost done' : '');
</script>

{#if devices.syncing && p}
  <section class="card" role="status" aria-label="Sync progress">
    <div class="head">
      <span class="title"><span class="spin"><Icon name="sync" size={16} /></span>Syncing with {name}</span>
    </div>
    <div class="bar" class:indeterminate={!sending}><div style:width="{pct}%"></div></div>
    <div class="current" title={step}>{step}</div>
    {#if detail}<div class="detail" title={detail}>{detail}</div>{/if}
    <div class="hint">Keep your {name} plugged in until this finishes.</div>
  </section>
{:else if report}
  <section class="card" role="status" aria-label="Sync result">
    <div class="head">
      <span class="title">
        <span class="ok" class:bad={report.errors.length > 0}><Icon name={report.errors.length ? 'close' : 'check'} size={16} /></span>
        {report.errors.length ? 'Sync finished with problems' : `${name} is up to date`}
      </span>
      <button class="icon-btn bare close" aria-label="Close" onclick={() => devices.dismissReport()}>
        <Icon name="close" size={16} />
      </button>
    </div>
    <div class="stats">
      <div><div class="num">{report.sent}</div><div class="cap">{report.sent === 1 ? 'book sent' : 'books sent'}</div></div>
      <div><div class="num">{report.progressUpdated}</div><div class="cap">progress updated</div></div>
      {#if report.errors.length}
        <div><div class="num bad">{report.errors.length}</div><div class="cap">{report.errors.length === 1 ? 'problem' : 'problems'}</div></div>
      {/if}
    </div>
    {#if report.errors.length}
      <details open>
        <summary>Show the {plural(report.errors.length, 'problem')}</summary>
        <ul class="mono">
          {#each report.errors as line, i (i)}
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
    border: 1px solid var(--teal-border);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.55);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    min-width: 0;
  }
  .spin,
  .ok {
    display: flex;
    color: var(--teal);
  }
  .ok.bad {
    color: var(--danger);
  }
  .spin {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .close {
    width: 28px;
    height: 28px;
    margin: -6px -6px -6px 0;
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--teal-track);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--teal);
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
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .detail {
    margin-top: -6px;
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hint {
    font-size: 12px;
    color: var(--muted);
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
  @media (prefers-reduced-motion: reduce) {
    .spin,
    .bar.indeterminate div {
      animation: none;
    }
  }
</style>
