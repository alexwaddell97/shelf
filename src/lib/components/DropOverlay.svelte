<script lang="ts">
  import { imports } from '$lib/stores/imports.svelte';
  import Icon from './Icon.svelte';
</script>

{#if imports.dragging}
  <div class="overlay" role="presentation">
    <div class="message">
      <Icon name="drop" size={40} />
      <strong>Drop to add to your library</strong>
      <span>.epub, .kepub, .mobi, .azw3 and .pdf files</span>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: var(--badge-bg);
    backdrop-filter: blur(4px);
    pointer-events: none;
  }
  .message {
    position: absolute;
    inset: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 2px dashed var(--accent);
    border-radius: var(--r4);
    color: var(--accent);
    text-align: center;
  }
  strong {
    font-size: 24px;
    letter-spacing: -0.01em;
    color: var(--text);
  }
  span {
    color: var(--muted);
  }
</style>
