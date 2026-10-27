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

  // A drag that starts inside the dialog (selecting text) and ends on the backdrop still
  // fires a click there; only a press that started on the backdrop should close.
  let pressedScrim = false;
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="scrim"
  onpointerdown={(e) => (pressedScrim = e.target === e.currentTarget)}
  onclick={(e) => {
    if (pressedScrim && e.target === e.currentTarget) onclose?.();
    pressedScrim = false;
  }}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby={labelledby}
    tabindex="-1"
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
    backdrop-filter: blur(10px) saturate(1.2);
  }
  .dialog {
    width: 100%;
    max-height: 100%;
    overflow-y: auto;
    border-radius: 28px;
    background: var(--glass-thick);
    backdrop-filter: var(--glass-filter);
    border: 0;
    box-shadow: var(--glass-shadow-lift);
  }
</style>
