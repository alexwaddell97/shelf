<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    id: string;
    label: string;
    values: string[];
    suggestions: string[];
    placeholder?: string;
    onchange: (next: string[]) => void;
  }

  let { id, label, values, suggestions, placeholder = 'Type and press Enter', onchange }: Props = $props();

  let text = $state('');
  let focused = $state(false);

  const lower = $derived(new Set(values.map((v) => v.toLowerCase())));
  const matches = $derived(
    suggestions
      .filter((s) => !lower.has(s.toLowerCase()) && s.toLowerCase().includes(text.trim().toLowerCase()))
      .slice(0, 6)
  );

  function add(raw: string) {
    const name = raw.trim();
    if (!name) return;
    // Reuse the existing spelling of a known label.
    const known = suggestions.find((s) => s.toLowerCase() === name.toLowerCase());
    const final = known ?? name;
    if (!lower.has(final.toLowerCase())) onchange([...values, final]);
    text = '';
  }

  function remove(v: string) {
    onchange(values.filter((x) => x !== v));
  }
</script>

<div class="wrap" role="group" aria-labelledby="{id}-label">
  <span id="{id}-label" class="lbl">{label}</span>
  <div class="box" class:focused>
    {#each values as v (v)}
      <span class="chip">
        {v}
        <button type="button" aria-label="Remove {v}" onclick={() => remove(v)}><Icon name="close" size={12} /></button>
      </span>
    {/each}
    <input
      {id}
      bind:value={text}
      {placeholder}
      autocomplete="off"
      spellcheck="false"
      aria-labelledby="{id}-label"
      onfocus={() => (focused = true)}
      onblur={() => {
        // Let a click on a suggestion land before the list disappears.
        setTimeout(() => (focused = false), 120);
        add(text);
      }}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ',') {
          if (text.trim()) {
            e.preventDefault();
            e.stopPropagation();
            add(text);
          }
        } else if (e.key === 'Backspace' && !text && values.length) {
          remove(values[values.length - 1]);
        }
      }}
    />
  </div>
  {#if focused && matches.length > 0}
    <div class="suggest" role="listbox" aria-label="{label} suggestions">
      {#each matches as s (s)}
        <button type="button" role="option" aria-selected="false" onmousedown={(e) => e.preventDefault()} onclick={() => add(s)}>
          {s}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .lbl {
    font-size: 12px;
    color: var(--muted);
  }
  .box {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-height: 40px;
    padding: 5px 8px;
    border-radius: var(--radius-m);
    border: 1px solid var(--border-control);
    background: var(--surface-sunken);
  }
  .box.focused {
    border-color: var(--accent);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 2px 10px;
    border-radius: 999px;
    background: var(--surface-chip);
    color: var(--text-soft);
    font-size: 13px;
  }
  .chip button {
    display: inline-flex;
    padding: 3px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--muted);
  }
  .chip button:hover {
    color: var(--text);
  }
  input {
    flex: 1 1 120px;
    min-width: 80px;
    height: 28px;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
  }
  input:focus-visible {
    outline: none;
  }
  .suggest {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 5;
    margin-top: 4px;
    display: flex;
    flex-direction: column;
    padding: 4px;
    border-radius: var(--radius-m);
    background: var(--surface-raised);
    border: 1px solid var(--border-control);
    box-shadow: var(--shadow-2);
  }
  .suggest button {
    padding: 7px 10px;
    border: 0;
    border-radius: var(--r1);
    background: transparent;
    color: var(--text);
    text-align: left;
  }
  .suggest button:hover {
    background: var(--surface-hover);
  }
</style>
