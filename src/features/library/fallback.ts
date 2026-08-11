import type { AccentView } from '@/shared/ipc/bindings';
import { hex } from './colours';

/**
 * What a film is drawn as when no frame could be taken from it.
 *
 * Every film gets a tile. A file the webview will not decode, a drive that was
 * unplugged before the capture reached it, a frame that came back blank: none
 * of those should leave a hole in the grid or a broken image icon in it, which
 * is what a poster fetched from somewhere else would give.
 *
 * Composed from the title rather than chosen at random, so the same film is the
 * same tile on every machine and after every restart.
 */
export interface Fallback {
  primary: string;
  pair: string;
  /** One or two letters, set large behind the title. */
  initials: string;
}

/** Words that start a title without saying anything about which one it is. */
const LEADING = new Set(['the', 'a', 'an', 'le', 'la', 'les', 'el', 'los', 'der', 'die', 'das']);

export function fallbackFor(title: string): Fallback {
  const hue = (hash(title.toLowerCase()) % 360000) / 1000;

  return {
    primary: hex(hue, 0.42, 0.52),
    // Far enough round to read as a pair rather than as a gradient that failed
    // to load, and always the same distance, so the tiles look like a set.
    pair: hex(hue + 42, 0.38, 0.46),
    initials: initialsOf(title),
  };
}

/**
 * The two colours a film is drawn in.
 *
 * Its own, taken from a frame, once there has been one to take. Until then, and
 * for anything that could not be decoded at all, the pair composed from its
 * title, so that a tile is never colourless and never changes colour twice.
 */
export function paletteOf(film: { title: string; accent: AccentView | null }): AccentView {
  if (film.accent) return film.accent;

  const { primary, pair } = fallbackFor(film.title);
  return { primary, pair };
}

/** The letters a title reduces to, at most two of them. */
export function initialsOf(title: string): string {
  const words = title
    .split(/[\s.:_-]+/)
    .map((word) => word.replace(/[^\p{L}\p{N}]/gu, ''))
    .filter((word) => word.length > 0);

  const meaningful = words.filter((word) => !LEADING.has(word.toLowerCase()));
  const chosen = meaningful.length > 0 ? meaningful : words;

  return chosen
    .slice(0, 2)
    .map((word) => word.slice(0, 1).toUpperCase())
    .join('');
}

/**
 * FNV-1a again, for the same reason it is used to name a poster file: the same
 * title has to give the same colour in a year's time, which rules out anything
 * the platform is free to change.
 */
function hash(text: string): number {
  let value = 0x811c9dc5;
  for (let at = 0; at < text.length; at += 1) {
    value ^= text.charCodeAt(at);
    // Multiplying by the prime through Math.imul keeps it a 32 bit integer,
    // which plain multiplication of a number this size would not.
    value = Math.imul(value, 0x01000193);
  }
  return value >>> 0;
}
