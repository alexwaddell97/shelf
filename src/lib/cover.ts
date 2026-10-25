// Deterministic generated covers, ported from the Cover component in the design mockup.
// The colours come from the sample books in the mockup; the motif is picked separately so
// the same palette doesn't always carry the same shape.

export type Motif = 'sun' | 'band' | 'block' | 'arch' | 'dot' | 'pillar';

export interface CoverStyle {
  bg: string;
  fg: string;
  mark: string;
  motif: Motif;
}

// [bg, fg, mark]
const PALETTE: [string, string, string][] = [
  ['#8E2D2D', '#F7EDE2', '#E9C9A0'],
  ['#3D2C5C', '#F3E9D2', '#E3B341'],
  ['#2F6F73', '#F2F0E9', '#1F4F52'],
  ['#1F3B5C', '#F1EDE4', '#F2B347'],
  ['#A9C3C6', '#142427', '#F4F1EA'],
  ['#111A2E', '#E9E6F2', '#E8B23C'],
  ['#3A2E2A', '#F0E6DA', '#C4553A'],
  ['#F2D06B', '#2B2205', '#E5703D'],
  ['#D9772B', '#1E1006', '#F6D7A7'],
  ['#3F5A36', '#F3EFD9', '#D9A441'],
  ['#E8E4DA', '#1C2A3A', '#8DB3D9'],
  ['#5B3A4A', '#F4EAE4', '#E8C9B8']
];

const MOTIF_NAMES: Motif[] = ['sun', 'band', 'block', 'arch', 'dot', 'pillar'];

/** [top, left, width, height, radius] */
export const MOTIFS: Record<Motif, [string, string, string, string, string]> = {
  sun: ['22%', '24%', '52%', '34.7%', '50%'],
  band: ['40%', '0', '100%', '8%', '0'],
  block: ['0', '64%', '36%', '44%', '0'],
  arch: ['20%', '20%', '60%', '36%', '999px 999px 0 0'],
  dot: ['26%', '62%', '22%', '14.7%', '50%'],
  pillar: ['0', '66%', '14%', '58%', '0']
};

/** FNV-1a, 32 bit. */
function hash(input: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    h ^= input.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

export function coverStyle(title: string, author: string): CoverStyle {
  const h = hash(`${title.trim().toLowerCase()}\u0000${author.trim().toLowerCase()}`);
  const [bg, fg, mark] = PALETTE[h % PALETTE.length];
  const motif = MOTIF_NAMES[Math.floor(h / PALETTE.length) % MOTIF_NAMES.length];
  return { bg, fg, mark, motif };
}

/** Title size in container-query width units, from title length and longest word. */
export function titleSizeCqw(title: string): number {
  const longest = Math.max(1, ...title.split(/\s+/).map((w) => w.length));
  const len = title.length;
  const byLength = len <= 6 ? 20 : len <= 12 ? 15.5 : len <= 22 ? 12.5 : 10.5;
  const byWord = 79 / (longest * 0.6);
  return Math.min(byLength, byWord);
}
