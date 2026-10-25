<script lang="ts">
  import { onMount } from 'svelte';
  import { forgetDevice, unqueue, type Book, type Device, type SupportLevel } from '$lib/api';
  import Cover from '$lib/components/Cover.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Toggle from '$lib/components/Toggle.svelte';
  import { clamp, formatBytes, formatDate, plural, relativeDate, statusLabel } from '$lib/format';
  import { devices } from '$lib/stores/devices.svelte';
  import { library } from '$lib/stores/library.svelte';
  import { nav } from '$lib/stores/nav.svelte';
  import { settings } from '$lib/stores/settings.svelte';
  import { attempt, toasts } from '$lib/stores/toast.svelte';

  // Only the Kobo driver exists in v0.1; everything else is on the roadmap.
  const FAMILIES = [
    { name: 'Kobo', models: 'Clara, Libra, Sage, Elipsa', format: 'KEPUB, converted automatically', level: 'Full support' },
    { name: 'PocketBook', models: 'Verse, Era, InkPad', format: 'EPUB', level: 'Planned' },
    { name: 'Tolino', models: 'Shine, Vision, Epos', format: 'EPUB', level: 'Planned' },
    { name: 'Kindle', models: 'Paperwhite, Scribe, Kindle', format: 'AZW3 via calibre (needs calibre)', level: 'Planned' },
    { name: 'Boox and Android readers', models: 'Over USB (MTP)', format: 'EPUB', level: 'Planned' },
    { name: 'KOReader', models: 'Any device running KOReader', format: 'EPUB · progress sync', level: 'Planned' }
  ];

  const SUPPORT_LABEL: Record<SupportLevel, string> = { full: 'Full support', beta: 'Beta driver', planned: 'Planned' };

  const TOGGLES = [
    {
      key: 'convertForDevice',
      label: 'Convert to the best format for each device',
      desc: 'Kobo gets kepub; other readers get EPUB.'
    },
    { key: 'autoSend', label: 'Auto-send new imports', desc: 'New books join the queue and go over the next time you sync.' },
    {
      key: 'backupDeviceDb',
      label: 'Back up the device database first',
      desc: 'Saves a copy of the e-reader’s database before Shelf writes anything.'
    }
  ] as const;

  onMount(() => {
    void devices.refresh(true);
  });

  const dev = $derived(devices.selected);
  const onDev = $derived(dev ? library.onDevice(dev.id) : []);
  const queued = $derived(dev ? library.queuedFor(dev.id) : []);
  const stats = $derived({
    onDevice: onDev.length,
    reading: onDev.filter((b) => b.status === 'reading').length,
    finished: onDev.filter((b) => b.status === 'finished').length
  });
  const onDevSorted = $derived(
    [...onDev].sort((a, b) => (b.lastRead ?? '').localeCompare(a.lastRead ?? '') || a.title.localeCompare(b.title))
  );
  const current = $derived(onDevSorted[0]);

  const usedBytes = $derived(
    dev?.totalBytes != null && dev.freeBytes != null ? Math.max(0, dev.totalBytes - dev.freeBytes) : null
  );
  const usedPct = $derived(usedBytes != null && dev?.totalBytes ? clamp((usedBytes / dev.totalBytes) * 100) : 0);

  const syncingThis = $derived(!!dev && devices.syncing && (devices.progress?.deviceId ?? dev.id) === dev.id);
  const syncPct = $derived(
    devices.progress && devices.progress.total > 0
      ? clamp(Math.round((devices.progress.done / devices.progress.total) * 100))
      : 0
  );
  const syncLabel = $derived(
    queued.length ? `Sync ${plural(queued.length, 'book')} to ${dev?.name ?? 'device'}` : 'Sync reading progress'
  );

  const fmtTag = (b: Book, d: Device) =>
    b.format.toLowerCase() === d.sendFormat.toLowerCase()
      ? b.format.toUpperCase()
      : `${b.format.toUpperCase()} → ${d.sendFormat.toUpperCase()}`;

  async function dropFromQueue(d: Device, b: Book) {
    const ok = await attempt(`Could not remove “${b.title}” from the queue`, async () => {
      await unqueue(d.id, b.id);
      return true;
    });
    if (ok) void library.refresh();
  }

  let confirmingForget = $state(false);
  let forgetting = $state(false);
  $effect(() => {
    dev?.id;
    confirmingForget = false;
  });

  async function forget(d: Device) {
    if (forgetting) return;
    forgetting = true;
    try {
      const ok = await attempt(`Could not forget ${d.name}`, async () => {
        await forgetDevice(d.id);
        return true;
      });
      if (ok) {
        toasts.success(`Shelf forgot ${d.name}. Your books stay in your library.`);
        confirmingForget = false;
        await devices.refresh();
        void library.refresh();
      }
    } finally {
      forgetting = false;
    }
  }
</script>

<section aria-labelledby="dev-h">
  <div class="head">
    <h1 id="dev-h" class="page-title">Devices</h1>
    {#if devices.list.length > 0}
      <div class="tabs" role="group" aria-label="Choose a device">
        {#each devices.list as d (d.id)}
          {@const on = dev?.id === d.id}
          <button class="tab" class:on aria-pressed={on} onclick={() => devices.select(d.id)}>
            <span class="dot" class:live={d.connected}></span>
            {d.name}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if !devices.loaded}
    <p class="muted">Looking for e-readers…</p>
  {:else if !dev}
    <div class="plug">
      <span class="plug-icon"><Icon name="device" size={30} /></span>
      <h2>Plug in your e-reader over USB</h2>
      <p>
        Shelf spots it automatically, then sends your queue and reads your progress back. Make sure the e-reader is
        switched on and, on a Kobo, choose “Connect” when it asks.
      </p>
      <button class="btn" onclick={() => devices.refresh(true)}>
        <Icon name="sync" size={16} />
        Check again
      </button>
    </div>
  {:else if dev.connected}
    <div class="device-card">
      <div class="art" aria-hidden="true">
        <div class="art-screen">
          {#if current}
            <div class="art-cover">
              <Cover title={current.title} author={current.author} coverPath={current.coverPath} />
            </div>
          {/if}
        </div>
      </div>

      <div class="device-info">
        <div>
          <h2>{dev.name}</h2>
          <p class="muted">Connected over USB{dev.mountPath ? ` at ${dev.mountPath}` : ''}</p>
        </div>
        {#if usedBytes != null && dev.totalBytes}
          <div class="storage">
            <div
              class="track"
              role="img"
              aria-label="{formatBytes(usedBytes)} used of {formatBytes(dev.totalBytes)}"
            >
              <div style:width="{usedPct}%"></div>
            </div>
            <div class="mono storage-label">
              {formatBytes(usedBytes)} used of {formatBytes(dev.totalBytes)}
              {#if dev.freeBytes != null}· {formatBytes(dev.freeBytes)} free{/if}
            </div>
          </div>
        {/if}
        <div class="stats">
          <div><div class="num">{stats.onDevice}</div><div class="cap">on device</div></div>
          <div><div class="num">{stats.reading}</div><div class="cap">reading</div></div>
          <div><div class="num">{stats.finished}</div><div class="cap">finished</div></div>
        </div>
      </div>

      <div class="sync">
        {#if syncingThis}
          <div class="syncing" role="status">
            <div class="sync-head">
              <span>Syncing…</span>
              {#if devices.progress && devices.progress.total > 0}
                <span class="mono">{syncPct}%</span>
              {/if}
            </div>
            <div class="bar" class:indeterminate={!devices.progress || devices.progress.total === 0}>
              <div style:width="{syncPct}%"></div>
            </div>
            <div class="sync-note">
              {#if devices.progress?.current}
                {devices.progress.current}
              {:else}
                Converting to {dev.sendFormat.toUpperCase()} and copying{dev.mountPath ? ` to ${dev.mountPath}` : ''}
              {/if}
            </div>
          </div>
        {:else if devices.justSynced === dev.id}
          <div class="synced" role="status">
            <Icon name="check" />
            All synced
          </div>
        {:else}
          <button class="btn primary big" onclick={() => devices.sync(dev.id)} disabled={devices.syncing}>
            <Icon name="sync" />
            {syncLabel}
          </button>
          <div class="sync-note center">Sends the queue, then reads progress back</div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="device-card offline">
      <div class="art off" aria-hidden="true"><div class="art-screen blank"></div></div>
      <div class="device-info">
        <div>
          <h2>{dev.name}</h2>
          <p class="muted">
            Not connected{dev.lastSeen ? ` · last seen ${formatDate(dev.lastSeen) || relativeDate(dev.lastSeen)}` : ''}
          </p>
        </div>
        <p class="soft">
          Plug it in over USB and Shelf picks up where it left off: anything queued for it is sent, and reading progress comes
          back.
        </p>
        <div class="chips">
          <span class="pill">Gets {dev.sendFormat.toUpperCase()}</span>
          <span class="pill" class:teal={dev.supportLevel === 'full'} class:amber={dev.supportLevel === 'beta'}>
            {SUPPORT_LABEL[dev.supportLevel]}
          </span>
        </div>
      </div>
      <div class="forget">
        {#if confirmingForget}
          <p class="forget-q">Forget {dev.name}? Your books stay in your library.</p>
          <div class="forget-btns">
            <button class="btn danger" onclick={() => forget(dev)} disabled={forgetting}>Forget it</button>
            <button class="btn" data-autofocus onclick={() => (confirmingForget = false)} disabled={forgetting}>Cancel</button>
          </div>
        {:else}
          <button class="btn danger" onclick={() => (confirmingForget = true)}>Forget this device</button>
        {/if}
      </div>
    </div>
  {/if}

  <div class="columns">
    {#if dev?.connected}
      <div class="lists">
        <div class="list">
          <h2 class="eyebrow">Waiting to send or update · {queued.length}</h2>
          {#if queued.length === 0}
            <p class="none">Nothing waiting. New imports join this queue when auto-send is on.</p>
          {/if}
          {#each queued as b (b.id)}
            <div class="line">
              <div class="thumb"><Cover title={b.title} author={b.author} coverPath={b.coverPath} /></div>
              <button class="line-text link" onclick={() => nav.openBook(b.id, { kind: 'devices' })}>
                <span class="line-title">{b.title}</span>
                <span class="line-sub">{b.author}</span>
              </button>
              {#if b.onDevices.includes(dev.id)}
                <span class="tag update">Waiting to update</span>
              {:else}
                <span class="tag mono">{fmtTag(b, dev)}</span>
              {/if}
              <button
                class="icon-btn bare"
                aria-label="Remove {b.title} from the queue"
                title="Remove from the queue"
                onclick={() => dropFromQueue(dev, b)}
              >
                <Icon name="close" size={16} />
              </button>
            </div>
          {/each}
        </div>

        <div class="list">
          <h2 class="eyebrow">On your {dev.name} · {stats.onDevice}</h2>
          {#if onDevSorted.length === 0}
            <p class="none">No books on this device yet.</p>
          {/if}
          {#each onDevSorted as b (b.id)}
            <button class="line btn-line" onclick={() => nav.openBook(b.id, { kind: 'devices' })} aria-label="{b.title} by {b.author}">
              <span class="thumb"><Cover title={b.title} author={b.author} coverPath={b.coverPath} /></span>
              <span class="line-text">
                <span class="line-title">{b.title}</span>
                <span class="line-sub">{b.author}</span>
              </span>
              <span class="progress-col">
                <span class="mini-bar"><span style:width="{b.progress}%"></span></span>
                <span class="mono progress-meta">
                  <span>{statusLabel(b)}</span>
                  <span>{b.lastRead ? relativeDate(b.lastRead) : 'Not opened'}</span>
                </span>
              </span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="panel settings">
      <h2 class="eyebrow">Sync settings</h2>
      {#if settings.value}
        {#each TOGGLES as t (t.key)}
          <div class="toggle-row">
            <div>
              <div class="toggle-label">{t.label}</div>
              <div class="toggle-desc">{t.desc}</div>
            </div>
            <Toggle
              pressed={settings.value[t.key]}
              label={t.label}
              onchange={(v) => settings.save({ [t.key]: v })}
            />
          </div>
        {/each}
      {:else}
        <p class="muted">Loading your settings…</p>
      {/if}
      {#if settings.tools && !settings.tools.kepubify}
        <p class="note">Install kepubify for faster Kobo books; until then books are sent as plain EPUB.</p>
      {/if}
    </div>
  </div>

  <div class="supported">
    <div>
      <h2 class="eyebrow">Supported e-readers</h2>
      <p class="help">
        Plug one in over USB and Shelf recognises it. Each kind of device has its own driver, so the community can add new ones.
        Kobo works today; the rest are planned.
      </p>
    </div>
    <div class="families">
      {#each FAMILIES as f (f.name)}
        <div class="family">
          <div class="family-head">
            <strong>{f.name}</strong>
            <span class="pill" class:teal={f.level === 'Full support'}>{f.level}</span>
          </div>
          <div class="family-models">{f.models}</div>
          <div class="family-format mono">{f.format}</div>
        </div>
      {/each}
    </div>
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 14px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-soft);
    font-weight: 500;
  }
  .tab.on {
    background: var(--text);
    border-color: var(--text);
    color: var(--bg);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--dot-off);
  }
  .dot.live {
    background: var(--teal);
  }

  .plug {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 48px 24px;
    text-align: center;
    border: 1px dashed var(--border);
    border-radius: 16px;
  }
  .plug-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 64px;
    height: 64px;
    border-radius: 16px;
    background: var(--surface-tile);
    color: var(--teal);
  }
  .plug h2 {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .plug p {
    max-width: 56ch;
    color: var(--muted);
  }

  .device-card {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 28px;
    padding: 24px;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .art {
    flex: none;
    width: 96px;
    height: 128px;
    padding: 8px 8px 8px 18px;
    border-radius: 12px;
    background: #0b0b0d;
    border: 1px solid var(--border-control);
    box-shadow: 0 14px 30px rgba(0, 0, 0, 0.5);
  }
  .art.off {
    padding: 8px;
    box-shadow: none;
  }
  .art-screen {
    width: 100%;
    height: 100%;
    border-radius: 3px;
    background: #d8d5cb;
    padding: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    filter: grayscale(1) contrast(0.9);
  }
  .art-screen.blank {
    background: var(--track);
    filter: none;
  }
  .art-cover {
    width: 100%;
  }
  .device-info {
    flex: 1 1 260px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .device-info h2 {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .soft {
    max-width: 60ch;
    color: var(--text-soft);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .storage {
    max-width: 360px;
  }
  .track {
    height: 6px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .track div {
    height: 100%;
    background: var(--teal);
  }
  .storage-label {
    margin-top: 6px;
    font-size: 11px;
    color: var(--faint);
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 22px;
  }
  .num {
    font-size: 22px;
    font-weight: 700;
  }
  .cap {
    font-size: 12px;
    color: var(--muted);
  }
  .sync {
    flex: 0 1 280px;
    min-width: 220px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .btn.big {
    height: 52px;
    border-radius: 12px;
    font-size: 15px;
    font-weight: 700;
    padding: 0 20px;
    white-space: normal;
    text-align: center;
  }
  .sync-note {
    font-size: 12px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .sync-note.center {
    text-align: center;
  }
  .syncing {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    border-radius: 12px;
    background: var(--surface-chip);
  }
  .sync-head {
    display: flex;
    justify-content: space-between;
    font-weight: 600;
  }
  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--track-strong);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease;
  }
  .bar.indeterminate div {
    width: 35% !important;
    animation: slide 1.1s ease-in-out infinite alternate;
  }
  @keyframes slide {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(185%);
    }
  }
  .synced {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    height: 52px;
    border-radius: 12px;
    background: var(--teal-bg);
    border: 1px solid var(--teal-border);
    color: var(--teal);
    font-weight: 700;
  }
  .forget {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 260px;
  }
  .forget-q {
    font-size: 13px;
    color: var(--text-soft);
  }
  .forget-btns {
    display: flex;
    gap: 8px;
  }

  .columns {
    display: flex;
    flex-wrap: wrap;
    gap: 24px;
    align-items: flex-start;
  }
  .lists {
    flex: 999 1 480px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .none {
    padding: 18px;
    border-radius: var(--radius-l);
    border: 1px dashed var(--border);
    color: var(--muted);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    border-radius: var(--radius-l);
    background: var(--surface-card);
    border: 1px solid var(--border-soft);
  }
  .btn-line {
    width: 100%;
    color: var(--text);
    text-align: left;
    transition: box-shadow 0.15s ease;
  }
  .btn-line:hover {
    box-shadow: inset 0 0 0 999px rgba(255, 255, 255, 0.05);
  }
  .thumb {
    display: block;
    width: 38px;
    flex: none;
  }
  .line-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
  }
  .line-title {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .line-sub {
    font-size: 13px;
    color: var(--muted);
  }
  .tag {
    font-size: 11px;
    color: var(--text-soft);
    padding: 3px 8px;
    border-radius: 6px;
    background: var(--surface-chip);
    white-space: nowrap;
  }
  .tag.update {
    background: var(--accent-bg);
    color: var(--accent);
    font-weight: 600;
  }
  .progress-col {
    flex: 0 1 160px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .mini-bar {
    height: 4px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .mini-bar span {
    display: block;
    height: 100%;
    background: var(--teal);
  }
  .progress-meta {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
    color: var(--faint);
    white-space: nowrap;
  }

  .settings {
    flex: 1 1 300px;
    min-width: 0;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .settings .eyebrow {
    margin-bottom: 8px;
  }
  .toggle-row {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 12px 0;
    border-top: 1px solid var(--border-faint);
  }
  .toggle-row > div {
    flex: 1;
    min-width: 0;
  }
  .toggle-label {
    font-weight: 600;
  }
  .toggle-desc {
    font-size: 13px;
    color: var(--muted);
  }
  .settings .note {
    margin-top: 8px;
  }

  .supported {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-top: 8px;
  }
  .help {
    margin-top: 6px;
    max-width: 70ch;
    color: var(--muted);
  }
  .families {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 12px;
  }
  .family {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px;
    border-radius: 12px;
    background: var(--surface-card);
    border: 1px solid var(--border-soft);
  }
  .family-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .family .pill {
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
  }
  .family-models {
    font-size: 13px;
    color: var(--muted);
  }
  .family-format {
    font-size: 11px;
    color: var(--faint);
  }
</style>
