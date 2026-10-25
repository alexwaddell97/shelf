<script lang="ts">
  interface Props {
    value: number | null;
    /** When given, the stars are clickable and report the new rating (null clears it). */
    onchange?: (next: number | null) => void;
    size?: number;
    label?: string;
  }

  let { value, onchange, size = 18, label = 'Rating' }: Props = $props();

  const STAR = 'M12 3.2l2.6 5.5 6 .8-4.4 4.2 1.1 6-5.3-2.9-5.3 2.9 1.1-6L3.4 9.5l6-.8z';
</script>

{#if onchange}
  <div class="stars" role="radiogroup" aria-label={label}>
    {#each [1, 2, 3, 4, 5] as n (n)}
      <button
        type="button"
        class="star"
        class:on={(value ?? 0) >= n}
        role="radio"
        aria-checked={value === n}
        aria-label="{n} {n === 1 ? 'star' : 'stars'}"
        onclick={() => onchange(value === n ? null : n)}
      >
        <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true"><path d={STAR}></path></svg>
      </button>
    {/each}
    {#if value}
      <button type="button" class="clear" onclick={() => onchange(null)}>Clear</button>
    {/if}
  </div>
{:else if value}
  <span class="stars" role="img" aria-label="{label}: {value} out of 5">
    {#each [1, 2, 3, 4, 5] as n (n)}
      <span class="star static" class:on={value >= n}>
        <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true"><path d={STAR}></path></svg>
      </span>
    {/each}
  </span>
{/if}

<style>
  .stars {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .star {
    display: inline-flex;
    padding: 2px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--border-dashed);
  }
  .star.on {
    color: var(--accent);
  }
  .star.on svg {
    fill: currentColor;
  }
  .star svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linejoin: round;
  }
  .star.static {
    padding: 0;
  }
  button.star:hover {
    color: var(--accent-hover);
  }
  .clear {
    margin-left: 8px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--muted);
    font-size: 12px;
    text-decoration: underline;
  }
</style>
