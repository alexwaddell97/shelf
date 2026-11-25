<script lang="ts">
  import { openUrl } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import { plural, timeAgo } from '$lib/format';
  import { describeApplied, goodreads } from '$lib/stores/goodreads.svelte';
  import { attempt } from '$lib/stores/toast.svelte';

  const EXPORT_PAGE = 'https://www.goodreads.com/review/import';

  const last = $derived(goodreads.status?.lastImport ?? null);
  const rows = $derived(
    last
      ? [
          { from: 'Want to Read', to: 'Wishlist', count: last.wantToRead },
          { from: 'Currently Reading', to: 'Reading', count: last.reading },
          { from: 'Read', to: 'Finished', count: last.read }
        ]
      : []
  );
</script>

<section aria-labelledby="acc-h">
  <div>
    <h1 id="acc-h" class="page-title">Accounts</h1>
    <p class="page-sub">
      Bring in a reading tracker to fill your wishlist and keep your reading history in step. Both are optional.
    </p>
  </div>

  <div class="account">
    <div class="head">
      <span class="mark" aria-hidden="true">g</span>
      <div class="text">
        <div class="name-row">
          <h2>Goodreads</h2>
          <span class="pill">Import</span>
        </div>
        <p>Want to Read fills your wishlist. Read and Currently Reading mark books in your library.</p>
      </div>
      {#if last}
        <span class="connected"><span class="dot"></span>Imported {timeAgo(last.importedAt)}</span>
      {/if}
    </div>

    {#if goodreads.status?.oldProfileLink}
      <div class="notice" role="note">
        <p>
          <strong>Your Goodreads profile link has stopped updating.</strong> Goodreads switched off the shelf feeds Shelf read
          (November 2026), and shelves now need signing in to see. Import your Goodreads export below instead. Your
          wishlist and reading history are kept.
        </p>
        <button class="btn sm ghost" onclick={() => goodreads.forgetProfileLink()}>Got it</button>
      </div>
    {/if}

    {#if last}
      <div class="rows">
        {#each rows as r (r.from)}
          <div class="row">
            <span class="from">{r.from}</span>
            <Icon name="arrow" size={16} />
            <span class="to">{r.to}</span>
            <span class="count">{plural(r.count, 'book')}</span>
          </div>
        {/each}
      </div>
      <p class="fine">
        From <span class="mono">{last.fileName}</span>. {describeApplied(last.applied).replace('Goodreads import: ', 'Last import: ')}
      </p>
    {/if}

    <ol class="steps">
      <li>
        <span class="step-text">On Goodreads, open <strong>My Books</strong>, then <strong>Import and export</strong>.</span>
        <button class="btn sm" onclick={() => attempt('Could not open Goodreads', () => openUrl(EXPORT_PAGE))}>
          <Icon name="external" size={15} />
          Open that page
        </button>
      </li>
      <li>
        <span class="step-text">
          Press <strong>Export Library</strong>. When the link appears (it can take a minute), download the file.
        </span>
      </li>
      <li>
        <span class="step-text">Choose the downloaded file here.</span>
        <button class="btn sm primary" onclick={() => goodreads.importCsv()} disabled={goodreads.importing}>
          {goodreads.importing ? 'Importing…' : last ? 'Import a newer export…' : 'Choose export file…'}
        </button>
      </li>
    </ol>

    <p class="fine">
      Import a fresh export whenever you like to bring Shelf up to date: books you've taken off Want to Read leave your
      wishlist too. Shelf only reads the file. Nothing is sent to Goodreads, and changes you make in Shelf don't go back
      to it.
    </p>
  </div>

  <div class="account soon-card">
    <div class="head">
      <span class="mark" aria-hidden="true">H</span>
      <div class="text">
        <div class="name-row">
          <h2>Hardcover</h2>
          <span class="pill teal">Two-way sync</span>
        </div>
        <p>
          Keeps your wishlist, reading progress and finished books in step, both ways. It can also import your Goodreads history.
        </p>
      </div>
      <span class="soon">Coming in a later version</span>
    </div>
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-width: 920px;
  }
  .account {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 22px;
    border-radius: var(--r3);
    background: var(--surface-card);
    border: 1px solid var(--border-soft);
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
  }
  .mark {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 44px;
    height: 44px;
    border-radius: var(--radius-l);
    background: var(--surface-tile);
    font-size: 18px;
    font-weight: 700;
  }
  .text {
    flex: 1 1 260px;
    min-width: 0;
  }
  .name-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  h2 {
    font-size: 18px;
    font-weight: 700;
  }
  .pill {
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
  }
  .text p {
    font-size: 13px;
    color: var(--muted);
  }
  .connected {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--teal);
    font-weight: 600;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--teal);
  }
  .fine {
    font-size: 12px;
    color: var(--faint);
    max-width: 75ch;
  }
  .rows {
    display: flex;
    flex-direction: column;
    border-radius: var(--r2);
    border: 1px solid var(--border-soft);
    overflow: hidden;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    background: var(--surface);
    color: var(--faint);
  }
  .row + .row {
    border-top: 1px solid var(--border-faint);
  }
  .from {
    flex: 0 1 180px;
    font-weight: 600;
    color: var(--text);
  }
  .to {
    flex: 0 1 140px;
    color: var(--text);
  }
  .count {
    flex: 1 1 100px;
    text-align: right;
    font-size: 13px;
    color: var(--muted);
  }
  .notice {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--r2);
    background: var(--accent-bg);
  }
  .notice p {
    flex: 1 1 320px;
    margin: 0;
    font-size: 13px;
    color: var(--text-soft);
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
    counter-reset: step;
  }
  .steps li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 14px;
    counter-increment: step;
  }
  .steps li::before {
    content: counter(step);
    display: grid;
    place-items: center;
    flex: none;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--fill);
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
  }
  .step-text {
    flex: 1 1 280px;
    font-size: 13px;
    color: var(--text-soft);
  }
  .soon {
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px dashed var(--border-dashed);
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }
</style>
