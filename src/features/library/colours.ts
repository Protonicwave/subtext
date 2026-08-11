import type { AccentView } from '@/shared/ipc/bindings';

/**
 * The colour pair a film is drawn in, taken from a frame of the film itself.
 *
 * This runs in the worker, on the same pixels that are about to be encoded, so
 * it costs one pass over an image that has already been decoded. It is here
 * rather than beside the worker because it is arithmetic over an array and can
 * be tested as such.
 *
 * The method is a weighted histogram over hue. Counting whole colours would
 * make the commonest one win, and on a still from a film the commonest colour
 * is almost always a dark neutral: a night interior would come out black, which
 * is no use as a glow. Weighting each pixel by how saturated it is asks a
 * different question, and one worth asking, which is what colour the frame
 * reads as.
 */

/** Fifteen degrees each, which is about as fine as hue is worth binning. */
const BINS = 24;

/** How far apart the two colours of a pair must be, in bins. */
const APART = 4;

/**
 * Pixels outside these are ignored: at the ends of the range a hue is a
 * rounding artefact rather than a colour anybody sees.
 */
const DARKEST = 0.06;
const LIGHTEST = 0.95;
const DULLEST = 0.1;

/**
 * What the colours are pulled into once they have been found.
 *
 * The accent is drawn as a glow behind a tile and as small text on a dark
 * ground. A colour taken straight from a frame meets neither: the frame is
 * usually darker than an interface wants, and a wash of it disappears. So the
 * hue is the film's and the rest is the design's.
 */
const SATURATION = { least: 0.34, most: 0.82 };
const LIGHTNESS = { least: 0.44, most: 0.7 };

/**
 * The pair for a frame, or nothing at all if it has no colour worth taking.
 *
 * A black and white film, or a frame that happens to be a fade, returns null
 * and keeps the default accent, which is the honest answer rather than a hue
 * invented from noise.
 */
export function accentFrom(pixels: Uint8ClampedArray): AccentView | null {
  const weight = new Float64Array(BINS);
  const saturation = new Float64Array(BINS);
  const lightness = new Float64Array(BINS);
  // Hue is an angle, so the bin at the top of the wheel holds values either
  // side of zero and averaging them arithmetically would land on cyan. Summed
  // as vectors and read back with atan2, it lands where it should.
  const across = new Float64Array(BINS);
  const up = new Float64Array(BINS);

  for (let at = 0; at + 3 < pixels.length; at += 4) {
    // A pixel the frame is only partly covering at the edges of a letterbox.
    if ((pixels[at + 3] ?? 0) < 128) continue;

    const colour = toHsl(pixels[at] ?? 0, pixels[at + 1] ?? 0, pixels[at + 2] ?? 0);
    if (colour.l < DARKEST || colour.l > LIGHTEST || colour.s < DULLEST) continue;

    const bin = Math.min(BINS - 1, Math.floor((colour.h / 360) * BINS));
    const angle = (colour.h * Math.PI) / 180;

    weight[bin] = (weight[bin] ?? 0) + colour.s;
    saturation[bin] = (saturation[bin] ?? 0) + colour.s * colour.s;
    lightness[bin] = (lightness[bin] ?? 0) + colour.s * colour.l;
    across[bin] = (across[bin] ?? 0) + colour.s * Math.cos(angle);
    up[bin] = (up[bin] ?? 0) + colour.s * Math.sin(angle);
  }

  const primary = heaviest(weight);
  if (primary === null) return null;

  const colourOf = (bin: number) => {
    const total = weight[bin] ?? 0;
    const hue = ((Math.atan2(up[bin] ?? 0, across[bin] ?? 0) * 180) / Math.PI + 360) % 360;
    return hex(hue, (saturation[bin] ?? 0) / total, (lightness[bin] ?? 0) / total);
  };

  const second = heaviest(weight, (bin) => distance(bin, primary) >= APART);

  return {
    primary: colourOf(primary),
    // A frame in one colour, which is common enough that it deserves an answer
    // rather than a pair of near identical hues. A third of the way round the
    // wheel from what is there reads as deliberate.
    pair: second === null ? shifted(colourOf(primary), 120) : colourOf(second),
  };
}

/** The bin carrying the most colour, among those a filter allows. */
function heaviest(weight: Float64Array, allowed?: (bin: number) => boolean): number | null {
  let found: number | null = null;
  let most = 0;

  for (let bin = 0; bin < weight.length; bin += 1) {
    const carried = weight[bin] ?? 0;
    if (carried <= most) continue;
    if (allowed && !allowed(bin)) continue;
    found = bin;
    most = carried;
  }
  return found;
}

/** How far apart two bins are, the short way round the wheel. */
function distance(one: number, other: number): number {
  const apart = Math.abs(one - other);
  return Math.min(apart, BINS - apart);
}

/** The same colour turned a given number of degrees around the wheel. */
function shifted(colour: string, degrees: number): string {
  const { h, s, l } = fromHex(colour);
  return hex((h + degrees) % 360, s, l);
}

interface Hsl {
  /** Degrees, from zero to 360. */
  h: number;
  /** Both from zero to one. */
  s: number;
  l: number;
}

/** A colour as hue, saturation and lightness, from three bytes. */
export function toHsl(red: number, green: number, blue: number): Hsl {
  const r = red / 255;
  const g = green / 255;
  const b = blue / 255;

  const most = Math.max(r, g, b);
  const least = Math.min(r, g, b);
  const range = most - least;
  const l = (most + least) / 2;

  if (range === 0) return { h: 0, s: 0, l };

  const s = range / (1 - Math.abs(2 * l - 1));
  let h: number;
  if (most === r) h = ((g - b) / range) % 6;
  else if (most === g) h = (b - r) / range + 2;
  else h = (r - g) / range + 4;

  return { h: (h * 60 + 360) % 360, s, l };
}

/**
 * A colour as `#rrggbb`, pulled into the range the interface can use.
 *
 * The clamping is the whole point of this rather than a plain conversion: what
 * comes out is always visible against the ground and always reads as a colour.
 */
export function hex(hue: number, saturation: number, lightness: number): string {
  const h = ((hue % 360) + 360) % 360;
  const s = clamp(saturation, SATURATION.least, SATURATION.most);
  const l = clamp(lightness, LIGHTNESS.least, LIGHTNESS.most);

  const chroma = (1 - Math.abs(2 * l - 1)) * s;
  const second = chroma * (1 - Math.abs(((h / 60) % 2) - 1));
  const offset = l - chroma / 2;

  const sixth = Math.floor(h / 60) % 6;
  const [r, g, b] = [
    [chroma, second, 0],
    [second, chroma, 0],
    [0, chroma, second],
    [0, second, chroma],
    [second, 0, chroma],
    [chroma, 0, second],
  ][sixth] ?? [0, 0, 0];

  return `#${byte((r ?? 0) + offset)}${byte((g ?? 0) + offset)}${byte((b ?? 0) + offset)}`;
}

function fromHex(colour: string): Hsl {
  const value = Number.parseInt(colour.slice(1), 16);
  // Three bytes packed into one integer, taken back out a byte at a time.
  return toHsl((value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff);
}

function byte(value: number): string {
  return Math.round(clamp(value, 0, 1) * 255)
    .toString(16)
    .padStart(2, '0');
}

function clamp(value: number, least: number, most: number): number {
  return Math.min(most, Math.max(least, value));
}
