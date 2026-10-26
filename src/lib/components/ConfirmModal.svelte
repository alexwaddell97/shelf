<script lang="ts">
  import type { Snippet } from 'svelte';
  import Modal from './Modal.svelte';

  interface Props {
    title: string;
    confirmLabel: string;
    busy?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
    children?: Snippet;
  }

  let { title, confirmLabel, busy = false, onconfirm, oncancel, children }: Props = $props();
</script>

<Modal labelledby="confirm-h" maxWidth="460px" onclose={oncancel}>
  <div class="body">
    <h2 id="confirm-h">{title}</h2>
    {#if children}
      <div class="text muted">{@render children()}</div>
    {/if}
    <div class="actions">
      <button class="btn" data-autofocus onclick={oncancel} disabled={busy}>Cancel</button>
      <button class="btn destructive" onclick={onconfirm} disabled={busy}>{confirmLabel}</button>
    </div>
  </div>
</Modal>

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
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
  }
  .btn.destructive {
    border-color: transparent;
    background: var(--danger);
    color: var(--on-danger);
    font-weight: 600;
  }
</style>
