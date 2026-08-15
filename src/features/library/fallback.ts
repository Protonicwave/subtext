import type { AccentView } from '@/shared/ipc/bindings';
import { hex } from './colours';

/**
 * The colours a film is drawn in before anything has been read off a picture.
 *
 * Every film gets a pair. A file the webview will not decode, a drive that was
 * unplugged before the poster was drawn, a film nobody has opened yet: none of
 * those should leave a colourless tile in the wall.
 *
 * Composed from the title rather than chosen at random, so the same film is the
 * same colour on every machine and after every restart.
 */
export interface Fallback {
  primary: string;
  pair: string;
}

export function fallbackFor(title: string): Fallback {
  const hue = (hash(title.toLowerCase()) % 360000) / 1000;

  return {
    primary: hex(hue, 0.42, 0.52),
    // Far enough round to read as a pair rather than as a gradient that failed
    // to load, and always the same distance, so the tiles look like a set.
    pair: hex(hue + 42, 0.38, 0.46),
  };
}

/**
 * The two colours a film is drawn in.
 *
 * Its own, taken from its cover, once there has been one to read. Until then,
 * and for anything that could not be decoded at all, the pair composed from its
 * title, so that a tile is never colourless and never changes colour twice.
 */
export function paletteOf(film: { title: string; accent: AccentView | null }): AccentView {
  if (film.accent) return film.accent;

  const { primary, pair } = fallbackFor(film.title);
  return { primary, pair };
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
