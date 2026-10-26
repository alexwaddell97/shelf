<script lang="ts">
  import { coverSrc } from '$lib/api';
  import { MOTIFS, coverStyle, titleSizeCqw } from '$lib/cover';

  interface Props {
    title: string;
    author: string;
    /** Local file path of an extracted cover (from a Book). */
    coverPath?: string | null;
    /** Remote cover image (from the catalogue or a wishlist item). */
    coverUrl?: string | null;
    /** Dimmed and desaturated, for wishlist tiles. */
    ghost?: boolean;
    /** Leave the top-right corner of the cover free for status badges. */
    reserveCorner?: boolean;
  }

  let { title, author, coverPath = null, coverUrl = null, ghost = false, reserveCorner = false }: Props = $props();

  const src = $derived.by(() => {
    if (coverPath) {
      try {
        return coverSrc(coverPath);
      } catch {
        return null;
      }
    }
    return coverUrl;
  });

  let failedSrc = $state<string | null>(null);
  let loadedSrc = $state<string | null>(null);

  /** Width ÷ height of the loaded image; covers are 2:3 (≈ 0.667). */
  let ratio = $state<number | null>(null);

  function loaded(img: HTMLImageElement) {
    loadedSrc = src;
    ratio = img.naturalHeight ? img.naturalWidth / img.naturalHeight : null;
  }

  /** Cached images can finish before the load handler is attached. */
  function watch(img: HTMLImageElement) {
    if (img.complete && img.naturalWidth > 0) loaded(img);
  }

  // Every cover sits in the same 2:3 frame. Close-enough images fill it; clearly
  // different shapes are shown whole, centred over a blurred copy, rather than cropped.
  const letterbox = $derived(ratio !== null && Math.abs(ratio - 2 / 3) > 0.06);
  const showImage = $derived(!!src && failedSrc !== src);

  const style = $derived(coverStyle(title, author));
  const motif = $derived(MOTIFS[style.motif]);
  const titleSize = $derived(titleSizeCqw(title).toFixed(2) + 'cqw');
</script>

<div
  class="cover"
  class:ghost
  class:corner={reserveCorner}
  style:--bg={style.bg}
  style:--fg={style.fg}
  style:--mark={style.mark}
>
  {#if showImage}
    {#if letterbox}
      <img class="backdrop" {src} alt="" aria-hidden="true" referrerpolicy="no-referrer" />
    {/if}
    <img
      {src}
      alt=""
      loading="lazy"
      referrerpolicy="no-referrer"
      class:loaded={loadedSrc === src}
      class:contain={letterbox}
      use:watch
      onload={(e) => loaded(e.currentTarget as HTMLImageElement)}
      onerror={() => (failedSrc = src)}
    />
    <span class="spine" aria-hidden="true"></span>
  {:else}
    <span
      class="mark"
      aria-hidden="true"
      style:top={motif[0]}
      style:left={motif[1]}
      style:width={motif[2]}
      style:height={motif[3]}
      style:border-radius={motif[4]}
    ></span>
    <span class="spine" aria-hidden="true"></span>
    <span class="author">{author}</span>
    <span class="title" style:font-size={titleSize}>{title}</span>
  {/if}
</div>

<style>
  .cover {
    position: relative;
    width: 100%;
    aspect-ratio: 2 / 3;
    container-type: inline-size;
    overflow: hidden;
    border-radius: var(--radius-s);
    background: var(--bg);
    color: var(--fg);
    box-shadow: var(--shadow-cover);
    font-family: var(--font-sans);
  }
  .cover.corner .author {
    right: 26%;
  }
  .cover.ghost {
    opacity: 0.5;
    filter: grayscale(0.45);
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    opacity: 0;
    transition: opacity 260ms var(--ease);
  }
  img.loaded {
    opacity: 1;
  }
  img.contain {
    object-fit: contain;
  }
  img.backdrop {
    opacity: 0.9;
    filter: blur(14px) saturate(1.15) brightness(0.92);
    transform: scale(1.18);
  }
  .mark {
    position: absolute;
    background: var(--mark);
  }
  .spine {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 5%;
    background: var(--cover-spine);
  }
  .author {
    position: absolute;
    top: 8%;
    left: 12%;
    right: 9%;
    font-size: 5.6cqw;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    line-height: 1.25;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
  }
  .title {
    position: absolute;
    left: 12%;
    right: 9%;
    bottom: 9%;
    font-weight: 700;
    line-height: 1;
    letter-spacing: -0.02em;
    overflow-wrap: anywhere;
  }
</style>
