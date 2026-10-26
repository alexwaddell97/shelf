<script lang="ts" module>
  const DEVICE = 'M7.2 2.5h9.6A2.2 2.2 0 0 1 19 4.7v14.6a2.2 2.2 0 0 1-2.2 2.2H7.2A2.2 2.2 0 0 1 5 19.3V4.7a2.2 2.2 0 0 1 2.2-2.2z';

  const ICONS = {
    library: ['M4 4h4v16H4z', 'M10 4h4v16h-4z', 'M15.5 5.5l3.4-1 3.6 14.6-3.4 1z'],
    bookmark: ['M7 3.5h10v17l-5-3.8-5 3.8z'],
    device: [DEVICE, 'M9 6.5h6M9 10h6M9 13.5h4'],
    'device-send': [DEVICE, 'M12 7v7M9 11l3 3 3-3'],
    link: ['M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1', 'M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1'],
    user: ['M12 4.5a3.5 3.5 0 1 0 0 7a3.5 3.5 0 0 0 0-7z', 'M5 20c1.2-3.6 4-5.5 7-5.5s5.8 1.9 7 5.5'],
    folder: ['M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z'],
    search: ['M11 4a7 7 0 1 0 0 14a7 7 0 0 0 0-14z', 'M20 20l-3.5-3.5'],
    grid: ['M4 4h7v7H4z', 'M13 4h7v7h-7z', 'M4 13h7v7H4z', 'M13 13h7v7h-7z'],
    list: ['M9 6h11M9 12h11M9 18h11', 'M4.5 6h.01M4.5 12h.01M4.5 18h.01'],
    plus: ['M12 5v14M5 12h14'],
    back: ['M19 12H5M11 6l-6 6 6 6'],
    arrow: ['M5 12h14M13 6l6 6-6 6'],
    external: ['M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5'],
    close: ['M6 6l12 12M18 6L6 18'],
    check: ['M5 12.5l4.5 4.5L19 7'],
    sync: ['M20 11a8 8 0 0 0-14.3-4.9L4 8M4 4v4h4M4 13a8 8 0 0 0 14.3 4.9L20 16M20 20v-4h-4'],
    drop: ['M12 4v10M8 10l4 4 4-4', 'M4 15v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3'],
    sliders: ['M4 7h8M18 7h2M4 17h2M12 17h8', 'M15 4.5a2.5 2.5 0 1 0 0 5a2.5 2.5 0 0 0 0-5z', 'M9 14.5a2.5 2.5 0 1 0 0 5a2.5 2.5 0 0 0 0-5z'],
    monitor: ['M3.5 4.5h17v11h-17z', 'M8 20h8M12 15.5V20'],
    sun: ['M12 8a4 4 0 1 0 0 8a4 4 0 0 0 0-8z', 'M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4'],
    moon: ['M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z'],
    layers: ['M12 3l9 5-9 5-9-5z', 'M3 13l9 5 9-5'],
    tag: ['M3 12V4h8l10 10-8 8z', 'M7.5 8.5h.01'],
    chevron: ['M9 6l6 6-6 6'],
    book: ['M5 4.5A1.5 1.5 0 0 1 6.5 3H19v15H6.5A1.5 1.5 0 0 0 5 19.5z', 'M5 19.5A1.5 1.5 0 0 0 6.5 21H19v-3']
  } satisfies Record<string, string[]>;

  export type IconName = keyof typeof ICONS;
</script>

<script lang="ts">
  let { name, size = 18 }: { name: IconName; size?: number } = $props();
</script>

<svg class="ic" width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
  {#each ICONS[name] as d (d)}
    <path {d}></path>
  {/each}
</svg>

<style>
  .ic {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    flex: none;
  }
</style>
