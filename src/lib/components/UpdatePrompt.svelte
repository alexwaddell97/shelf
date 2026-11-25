<script lang="ts">
  import { updates } from '$lib/stores/updates.svelte';
  import Icon from './Icon.svelte';

  // Bottom right, with the other floating panels: a new version is out. "Not now" hides it
  // for this version; Settings keeps offering it.
  const next = $derived(updates.available);
  const pct = $derived(updates.progress === null ? null : Math.round(updates.progress * 100));
</script>

{#if updates.prompting && next}
  <div class="card" role="dialog" aria-labelledby="update-h" aria-describedby="update-sub">
    <div class="head">
      <span class="badge" aria-hidden="true"><Icon name="drop" size={18} /></span>
      <div class="text">
        <h2 id="update-h">Shelf {next.version} is available</h2>
        <p id="update-sub">
          {#if updates.phase === 'installing'}
            Installing… Shelf will restart by itself.
          {:else if updates.phase === 'downloading'}
            Downloading{pct === null ? '…' : ` · ${pct}%`}
          {:else}
            You have {updates.current || 'an older version'}.
          {/if}
        </p>
      </div>
      {#if !updates.busy}
        <button class="icon-btn bare close" aria-label="Not now" title="Not now" onclick={() => updates.dismiss()}>
          <Icon name="close" size={16} />
        </button>
      {/if}
    </div>

    {#if updates.busy}
      <div class="bar" class:indeterminate={pct === null}><div style:width="{pct ?? 35}%"></div></div>
    {:else}
      {#if next.notes}
        <div class="notes">
          <div class="notes-label">What's new</div>
          <p>{next.notes}</p>
        </div>
      {/if}
      <div class="actions">
        <button class="btn sm" onclick={() => updates.dismiss()}>Not now</button>
        <button class="btn sm primary" onclick={() => updates.install()}>Update now</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .card {
    width: 340px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border-radius: 22px;
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--glass-shadow-lift);
    animation: rise var(--dur) var(--ease);
  }
  .head {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }
  .badge {
    flex: none;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 12px;
    background: var(--accent);
    color: var(--on-accent);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
  }
  p {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--muted);
  }
  .close {
    width: 28px;
    height: 28px;
    margin: -4px -6px 0 0;
  }
  .notes {
    max-height: 120px;
    overflow-y: auto;
    padding: 10px 12px;
    border-radius: 14px;
    background: var(--fill);
  }
  .notes-label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--faint);
  }
  .notes p {
    margin-top: 4px;
    color: var(--text-soft);
    white-space: pre-line;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width 0.2s linear;
  }
  .bar.indeterminate div {
    animation: slide 1.2s var(--ease) infinite alternate;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(290%);
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card,
    .bar.indeterminate div {
      animation: none;
    }
  }
</style>
