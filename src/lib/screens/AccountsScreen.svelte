<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { plural, timeAgo } from '$lib/format';
  import { goodreads } from '$lib/stores/goodreads.svelte';

  let input = $state('');
  let confirmingDisconnect = $state(false);

  const account = $derived(goodreads.account);
  const rows = $derived(
    account
      ? [
          { from: 'Want to Read', to: 'Wishlist', count: account.wantToRead },
          { from: 'Currently Reading', to: 'Reading', count: account.reading },
          { from: 'Read', to: 'Finished', count: account.read }
        ]
      : []
  );

  async function connect(e: SubmitEvent) {
    e.preventDefault();
    if (!input.trim()) {
      goodreads.error = 'Paste your Goodreads profile link first.';
      return;
    }
    if (await goodreads.connect(input)) input = '';
  }
</script>

<section aria-labelledby="acc-h">
  <div>
    <h1 id="acc-h" class="page-title">Accounts</h1>
    <p class="page-sub">
      Connect a reading tracker to fill your wishlist and keep your reading history in step. Both are optional.
    </p>
  </div>

  <div class="account">
    <div class="head">
      <span class="mark" aria-hidden="true">g</span>
      <div class="text">
        <div class="name-row">
          <h2>Goodreads</h2>
          <span class="pill">Read-only</span>
        </div>
        <p>Want to Read fills your wishlist. Read and Currently Reading mark books in your library.</p>
      </div>
      {#if account}
        <span class="connected"><span class="dot"></span>Connected as {account.name ?? `Goodreads user ${account.userId}`}</span>
      {/if}
    </div>

    {#if !account}
      {#if goodreads.connecting}
        <div class="checking" role="status">
          <span class="spin ring"></span>
          Checking your shelves…
        </div>
      {:else}
        <form class="connect" onsubmit={connect}>
          <div class="field grow">
            <label for="gr-user">Your Goodreads profile link</label>
            <input
              id="gr-user"
              class="input big"
              bind:value={input}
              placeholder="goodreads.com/user/show/12345-yourname"
              autocomplete="off"
              spellcheck="false"
              aria-describedby={goodreads.error ? 'gr-error' : undefined}
              aria-invalid={goodreads.error ? true : undefined}
            />
          </div>
          <button type="submit" class="btn primary">Connect</button>
        </form>
        {#if goodreads.error}
          <p id="gr-error" class="error" role="alert">{goodreads.error}</p>
        {/if}
      {/if}
      <p class="fine">
        Read-only, and no password needed: Shelf reads your public shelf feeds. Your profile needs to be public. Nothing is
        sent to Goodreads, and changes you make in Shelf don't go back to it.
      </p>
    {:else}
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
      <div class="footer">
        <span class="fine grow">
          Checks every hour · last checked {account.lastChecked ? timeAgo(account.lastChecked) : 'not yet'}
        </span>
        <button class="btn sm" onclick={() => goodreads.check()} disabled={goodreads.checking}>
          {goodreads.checking ? 'Checking…' : 'Check now'}
        </button>
        {#if confirmingDisconnect}
          <button class="btn sm danger" onclick={() => { confirmingDisconnect = false; void goodreads.disconnect(); }}>
            Yes, disconnect
          </button>
          <button class="btn sm ghost" onclick={() => (confirmingDisconnect = false)}>Cancel</button>
        {:else}
          <button class="btn sm ghost danger" onclick={() => (confirmingDisconnect = true)}>Disconnect</button>
        {/if}
      </div>
    {/if}

    <div class="csv">
      <div class="csv-text">
        <strong>Import a Goodreads export (CSV)</strong>
        <span>
          Gets your full history, not just recent shelves. On Goodreads: My Books, then Import and export, then Export
          library.
        </span>
      </div>
      <button class="btn sm" onclick={() => goodreads.importCsv()} disabled={goodreads.importing}>
        {goodreads.importing ? 'Importing…' : 'Choose CSV file…'}
      </button>
    </div>
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
  .connect {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 12px;
  }
  .grow {
    flex: 1 1 320px;
  }
  .input.big {
    height: 44px;
  }
  .connect .btn {
    height: 44px;
    padding: 0 20px;
  }
  .error {
    color: var(--danger);
    font-size: 13px;
    margin-top: -6px;
    word-break: break-word;
  }
  .checking {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-soft);
  }
  .ring {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--border-dashed);
    border-top-color: var(--accent);
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
  .footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .footer .grow {
    flex: 1 1 200px;
  }
  .csv {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
    padding-top: 16px;
    border-top: 1px solid var(--border-faint);
  }
  .csv-text {
    flex: 1 1 280px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .csv-text span {
    font-size: 13px;
    color: var(--muted);
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
