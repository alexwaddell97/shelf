<script lang="ts">
  import type { Snippet } from 'svelte';
  import { trapFocus } from '$lib/actions';

  interface Props {
    /** id of the heading inside that names the dialog */
    labelledby: string;
    /** Called on Escape or a click on the scrim. Leave out to make the dialog non-dismissable that way. */
    onclose?: () => void;
    maxWidth?: string;
    children: Snippet;
  }

  let { labelledby, onclose, maxWidth = '660px', children }: Props = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="scrim"
  onclick={(e) => {
    if (e.target === e.currentTarget) onclose?.();
  }}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby={labelledby}
    style:max-width={maxWidth}
    use:trapFocus={onclose}
  >
    {@render children()}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    overflow-y: auto;
    background: var(--scrim);
    backdrop-filter: blur(6px);
  }
  .dialog {
    width: 100%;
    max-height: 100%;
    overflow-y: auto;
    border-radius: 20px;
    background: var(--surface-modal);
    border: 1px solid var(--border);
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6);
  }
</style>
