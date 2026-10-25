<script lang="ts">
  interface Props {
    pressed: boolean;
    /** Accessible name; the button has no visible text of its own. */
    label: string;
    disabled?: boolean;
    onchange: (next: boolean) => void;
  }

  let { pressed, label, disabled = false, onchange }: Props = $props();
</script>

<button
  type="button"
  class="toggle"
  class:on={pressed}
  aria-pressed={pressed}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!pressed)}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    flex: none;
    position: relative;
    width: 46px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: 999px;
    background: var(--track-strong);
    transition: background 0.15s ease;
  }
  .toggle.on {
    background: var(--accent);
  }
  .knob {
    position: absolute;
    top: 4px;
    left: 4px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--faint);
    transition:
      left 0.15s ease,
      background 0.15s ease;
  }
  .on .knob {
    left: 22px;
    background: var(--on-accent);
  }
</style>
