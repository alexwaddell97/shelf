<script lang="ts">
  import { onMount } from 'svelte';
  import { basename, plural } from '$lib/format';
  import { calibre } from '$lib/stores/calibre.svelte';
  import { devices } from '$lib/stores/devices.svelte';
  import { imports } from '$lib/stores/imports.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';
  import Toggle from './Toggle.svelte';

  const STEPS = [
    ['Welcome to Shelf', 'A fast home for your ebooks that keeps your e-reader in sync. A few quick steps, and you can change all of it later.'],
    ['Your e-reader', 'Shelf picks the right format and sync for the device you read on.'],
    ['Where new books come from', 'Shelf watches these folders and imports any ebook that lands in them.'],
    ['Bring your books', 'Start with the books you already have.'],
    ["You're all set", 'Here is what Shelf will do. Everything is under Sources and Devices if you want to change it.']
  ] as const;
  const LAST = STEPS.length - 1;

  let step = $state(0);
  let bookChoice = $state<'calibre' | 'files' | 'empty' | null>(null);
  let picked = $state(0);
  let picking = $state(false);
  let finishing = $state(false);

  const folders = $derived(settings.value?.watchFolders ?? []);
  const autoSend = $derived(settings.value?.autoSend ?? false);
  const found = $derived(devices.connected);
  const remembered = $derived(devices.list.filter((d) => !d.connected));

  onMount(() => {
    void devices.refresh(true);
    void calibre.detect(true);
  });

  async function chooseFiles() {
    if (picking) return;
    bookChoice = 'files';
    picking = true;
    try {
      const n = await imports.pickAndImport();
      if (n > 0) picked += n;
    } finally {
      picking = false;
    }
  }

  async function finish() {
    if (finishing) return;
    finishing = true;
    try {
      await settings.finishSetup();
    } finally {
      finishing = false;
    }
  }

  function next() {
    if (step >= LAST) void finish();
    else step += 1;
  }
</script>

<Modal labelledby="setup-h">
  <div class="body">
    <div class="meta">
      <div class="dots" aria-hidden="true">
        {#each STEPS as _, i (i)}
          <span class="seg" class:on={i <= step}></span>
        {/each}
      </div>
      <span class="counter mono">Step {step + 1} of {STEPS.length}</span>
    </div>

    <div>
      <h2 id="setup-h">{STEPS[step][0]}</h2>
      <p class="sub">{STEPS[step][1]}</p>
    </div>

    {#if step === 0}
      <div class="trio">
        <div class="tile"><strong>Your library</strong><span>Covers, search and sorting, fast.</span></div>
        <div class="tile"><strong>Your e-reader</strong><span>Right format, one-click sync.</span></div>
        <div class="tile"><strong>Your reading</strong><span>Progress and wishlist, in step.</span></div>
      </div>
    {:else if step === 1}
      <div class="stack">
        {#if found.length > 0}
          {#each found as d (d.id)}
            <div class="found">
              <span class="pulse dot"></span>
              Found {d.name}{d.mountPath ? ` on ${d.mountPath}` : ''}
            </div>
          {/each}
        {:else}
          <div class="none">
            <span class="dot off"></span>
            <div>
              <strong>No e-reader detected yet</strong>
              <span>Plug it in over USB, switch it on, and on a Kobo tap “Connect” when it asks. It shows up here within a few seconds. You can also skip this and connect it later.</span>
            </div>
          </div>
        {/if}
        {#if remembered.length > 0}
          <p class="small">Remembered from before: {remembered.map((d) => d.name).join(', ')}</p>
        {/if}
        <p class="small">
          Kobo e-readers are fully supported today. Other makes are planned, and Shelf will tell you when it recognises one.
        </p>
      </div>
    {:else if step === 2}
      <div class="stack">
        {#each folders as f (f)}
          <div class="folder">
            <Icon name="folder" size={20} />
            <div class="folder-text">
              <strong>{basename(f)}</strong>
              <span class="mono" title={f}>{f}</span>
            </div>
            <button class="icon-btn bare" aria-label="Stop watching {basename(f)}" onclick={() => settings.removeWatchFolder(f)}>
              <Icon name="close" size={16} />
            </button>
          </div>
        {:else}
          <p class="small">No folders are being watched yet.</p>
        {/each}
        <button class="btn sm dashed" onclick={() => settings.addWatchFolder()}>
          <Icon name="plus" size={16} />
          Add {folders.length ? 'another ' : 'a '}folder
        </button>
        <div class="row-toggle">
          <div>
            <strong>Send new books to my e-reader automatically</strong>
            <span>They go over the next time you connect it.</span>
          </div>
          <Toggle pressed={autoSend} label="Send new books automatically" onchange={(v) => settings.save({ autoSend: v })} />
        </div>
      </div>
    {:else if step === 3}
      <div class="stack tight">
        {#if calibre.found}
          <button
            class="choice"
            class:selected={bookChoice === 'calibre'}
            aria-pressed={bookChoice === 'calibre'}
            disabled={calibre.running || calibre.inspecting}
            onclick={() => {
              bookChoice = 'calibre';
              if (calibre.found) void calibre.prepare(calibre.found);
            }}
          >
            <span class="ring"></span>
            <span class="choice-text">
              <strong>Import my calibre library</strong>
              <span>
                {#if calibre.running}
                  Importing… progress shows bottom right.
                {:else if calibre.report}
                  Imported {plural(calibre.report.imported, 'book')}.
                {:else}
                  Books, covers and series from <span class="mono">{calibre.found}</span>. calibre itself is left untouched.
                {/if}
              </span>
            </span>
          </button>
        {/if}
        <button
          class="choice"
          class:selected={bookChoice === 'files'}
          aria-pressed={bookChoice === 'files'}
          disabled={picking}
          onclick={chooseFiles}
        >
          <span class="ring"></span>
          <span class="choice-text">
            <strong>Choose ebook files…</strong>
            <span>
              {#if picked > 0}
                {plural(picked, 'file')} sent to import. Progress shows bottom right.
              {:else}
                Pick .epub, .kepub, .mobi, .azw3 or .pdf files from anywhere on this PC.
              {/if}
            </span>
          </span>
        </button>
        <button class="choice" class:selected={bookChoice === 'empty'} aria-pressed={bookChoice === 'empty'} onclick={() => (bookChoice = 'empty')}>
          <span class="ring"></span>
          <span class="choice-text">
            <strong>Start empty</strong>
            <span>Add books later by dropping files on the window.</span>
          </span>
        </button>
      </div>
    {:else}
      <ul class="summary">
        <li>
          <Icon name="device" size={18} />
          {#if found.length}Connected: {found.map((d) => d.name).join(', ')}{:else}No e-reader connected yet; plug one in any time{/if}
        </li>
        <li>
          <Icon name="folder" size={18} />
          {#if folders.length}Watching {plural(folders.length, 'folder')} for new books{:else}Not watching any folders{/if}
        </li>
        <li>
          <Icon name="device-send" size={18} />
          {autoSend ? 'New books are queued for your e-reader automatically' : 'You choose which books go to your e-reader'}
        </li>
        <li>
          <Icon name="library" size={18} />
          {#if picked > 0}{plural(picked, 'file')} being imported{:else}Your library starts empty{/if}
        </li>
      </ul>
    {/if}
  </div>

  <div class="footer">
    <button class="skip" onclick={finish} disabled={finishing}>Skip setup</button>
    <div class="nav-btns">
      {#if step > 0}
        <button class="btn" onclick={() => (step -= 1)}>Back</button>
      {/if}
      <button class="btn primary" data-autofocus onclick={next} disabled={finishing}>
        {step === 0 ? 'Get started' : step === LAST ? 'Finish' : 'Continue'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .body {
    padding: 28px 32px 12px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .dots {
    display: flex;
    gap: 6px;
  }
  .seg {
    width: 28px;
    height: 4px;
    border-radius: 999px;
    background: var(--track-strong);
  }
  .seg.on {
    background: var(--accent);
  }
  .counter {
    font-size: 11px;
    color: var(--faint);
  }
  h2 {
    margin: 0;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .sub {
    margin: 6px 0 0;
    color: var(--muted);
    max-width: 60ch;
  }
  .trio {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    padding: 8px 0 12px;
  }
  .tile {
    flex: 1 1 160px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px;
    border-radius: 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }
  .tile span {
    font-size: 13px;
    color: var(--muted);
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 12px;
  }
  .stack.tight {
    gap: 8px;
  }
  .found {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--teal-bg);
    border: 1px solid var(--teal-border);
    color: var(--teal);
    font-weight: 600;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--teal);
  }
  .dot.off {
    margin-top: 6px;
    background: var(--dot-off);
  }
  .none {
    display: flex;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }
  .none div {
    display: flex;
    flex-direction: column;
  }
  .none span {
    font-size: 13px;
    color: var(--muted);
  }
  .small {
    margin: 0;
    font-size: 13px;
    color: var(--muted);
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }
  .folder-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .folder-text span {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dashed {
    align-self: flex-start;
    border-style: dashed;
    border-color: var(--border-dashed);
    background: transparent;
  }
  .row-toggle {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--border-soft);
  }
  .row-toggle > div {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .row-toggle span {
    font-size: 13px;
    color: var(--muted);
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 64px;
    padding: 12px 14px;
    border-radius: 12px;
    border: 1.5px solid var(--border-control);
    background: var(--surface-raised);
    color: var(--text);
    text-align: left;
  }
  .choice.selected {
    border-color: var(--accent);
    background: var(--accent-bg-strong);
  }
  .ring {
    flex: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--border-dashed);
    background: var(--surface-modal);
  }
  .selected .ring {
    border: 5px solid var(--accent);
  }
  .choice-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .choice-text span {
    font-size: 13px;
    color: var(--muted);
  }
  .summary {
    list-style: none;
    margin: 0;
    padding: 0 0 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .summary li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
  }
  .footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 32px;
    border-top: 1px solid var(--border-soft);
    background: var(--surface-footer);
  }
  .skip {
    height: 40px;
    padding: 0 4px;
    border: 0;
    background: transparent;
    color: var(--muted);
  }
  .nav-btns {
    display: flex;
    gap: 10px;
  }
  .btn.primary {
    padding: 0 22px;
    font-weight: 700;
  }
</style>
