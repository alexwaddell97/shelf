<script lang="ts">
  import { details } from '$lib/stores/details.svelte';
  import Icon from './Icon.svelte';

  // Bottom right while "Fill in missing details" works through a selection.
  const p = $derived(details.progress);
  const pct = $derived(p && p.total ? Math.round((p.done / p.total) * 100) : 0);
</script>

{#if details.running && p}
  <section class="card" role="status" aria-label="Filling in details">
    <span class="title"><span class="spin"><Icon name="search" size={16} /></span>Filling in details · {p.done + 1 > p.total ? p.total : p.done + 1} of {p.total}</span>
    <div class="bar"><div style:width="{pct}%"></div></div>
    {#if p.current}<div class="current" title={p.current}>{p.current}</div>{/if}
    <div class="hint">Only empty fields and missing covers are filled in.</div>
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
    border-radius: 22px;
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--glass-shadow);
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
  }
  .spin {
    display: inline-flex;
    color: var(--accent);
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
  }
  .current {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hint {
    font-size: 12px;
    color: var(--muted);
  }
</style>
