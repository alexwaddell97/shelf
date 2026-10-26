<script lang="ts">
  // A paragraph cut to a few lines, with "Read more" to expand it in place. The link
  // only appears when the text is actually longer than that.

  interface Props {
    text: string;
    lines?: number;
  }

  let { text, lines = 4 }: Props = $props();

  let expanded = $state(false);
  let overflows = $state(false);
  let el = $state<HTMLParagraphElement | null>(null);

  function measure() {
    if (el && !expanded) overflows = el.scrollHeight > el.clientHeight + 1;
  }

  $effect(() => {
    void text;
    void lines;
    measure();
  });

  // Re-check when the width changes (window resize, panel opening).
  $effect(() => {
    if (!el) return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  });
</script>

<div class="clamp-text">
  <p bind:this={el} class:clamped={!expanded} style:--lines={lines}>{text}</p>
  {#if overflows || expanded}
    <button class="more" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>
      {expanded ? 'Show less' : 'Read more'}
    </button>
  {/if}
</div>

<style>
  p {
    margin: 0;
    white-space: pre-line;
  }
  .clamped {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: var(--lines);
    line-clamp: var(--lines);
    overflow: hidden;
  }
  .more {
    margin-top: 6px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .more:hover {
    text-decoration: underline;
  }
</style>
