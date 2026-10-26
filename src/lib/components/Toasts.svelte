<script lang="ts">
  import { toasts } from '$lib/stores/toast.svelte';
</script>

<div class="stack" aria-live="polite">
  {#each toasts.items as t (t.id)}
    <div class="toast {t.kind}" role={t.kind === 'error' ? 'alert' : 'status'}>
      <div class="body">
        <span class="text">{t.text}</span>
        {#if t.detail}
          <span class="detail mono">{t.detail}</span>
        {/if}
      </div>
      {#if t.action}
        {@const action = t.action}
        <button
          class="action"
          onclick={() => {
            toasts.dismiss(t.id);
            action.run();
          }}>{action.label}</button>
      {/if}
      <button class="close" aria-label="Dismiss message" onclick={() => toasts.dismiss(t.id)}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18"></path></svg>
      </button>
    </div>
  {/each}
</div>

<style>
  .action {
    flex: none;
    height: 30px;
    padding: 0 12px;
    border: 0;
    border-radius: 8px;
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .stack {
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    z-index: 60;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: max-content;
    max-width: min(560px, calc(100% - 32px));
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    max-width: 100%;
    padding: 12px 12px 12px 16px;
    border-radius: 18px;
    background: var(--glass-ink);
    backdrop-filter: var(--glass-filter);
    color: var(--toast-text);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.14),
      0 12px 30px rgba(0, 0, 0, 0.22);
    animation: rise var(--dur) var(--ease);
  }
  .toast.error {
    background: color-mix(in srgb, var(--toast-error-bg) 86%, transparent);
    color: var(--toast-error-text);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .text {
    font-weight: 600;
  }
  .detail {
    font-size: 12px;
    color: var(--toast-mono);
    word-break: break-all;
    white-space: pre-wrap;
  }
  .error .detail {
    color: var(--toast-error-mono);
  }
  .close {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: var(--r1);
    background: transparent;
    color: inherit;
    opacity: 0.6;
  }
  .close:hover {
    opacity: 1;
  }
  .close svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
