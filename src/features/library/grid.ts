import type { Settings } from '@/shared/settings/schema';

/**
 * How the poster grid lays out at a given width.
 *
 * Kept apart from the component because it is arithmetic and nothing else, and
 * because the virtualiser needs the answer before anything is drawn: it works in
 * rows, so it has to know how many tiles are in one.
 */

/** A tile is at least this wide, and the columns stretch to fill what is left. */
export const MIN_TILE = 176;

/** The gap between tiles, matching the value the stylesheet uses. */
export const TILE_GAP = 20;

/**
 * How wide a tile is on a shelf.
 *
 * Fixed rather than stretched, unlike the wall: a rail is scrolled rather than
 * wrapped, so there is no width to divide up, and every rail on the screen
 * should have tiles of the same size whatever is in it. Given to the stylesheet
 * as a custom property so that this is the only place it is written down.
 */
export const RAIL_TILE = 168;

/** The gap between tiles on a rail, which is tighter than the wall's. */
export const RAIL_GAP = 16;

/** Room under each tile for the title and the line beneath it. */
export const CAPTION = 44;

/**
 * The same, for a tile on a rail.
 *
 * More than the wall reserves. A rail hides what runs past it, to keep a
 * vertical scrollbar out of a row that scrolls sideways, so a caption with too
 * little room is cut in half rather than merely sitting close to the row below.
 */
export const RAIL_CAPTION = 58;

/**
 * Two by three, the shape a film poster has been for a century.
 *
 * The frames captured from a film are sixteen by nine, so they are cropped to
 * this rather than letterboxed into it. A wall of tall tiles with black bars
 * down both sides is worse than losing the edges of each frame.
 */
export const TILE_RATIO = 3 / 2;

/**
 * What each of the three tile sizes does to the two widths above.
 *
 * Applied to the widths rather than drawn as three sets of numbers, so that the
 * shape of a tile, the gap beside it and the room under it all follow from one
 * place. Small is about a fifth off and large about a quarter on, which is the
 * range over which a wall still reads as a wall: less than that is a difference
 * nobody would notice, and more is a different screen.
 */
const SCALES = { small: 0.8, medium: 1, large: 1.25 } as const;

export type TileSize = Settings['tileSize'];

/** How wide a tile on the wall is at least, at the size that was chosen. */
export function wallTileFor(size: TileSize): number {
  return Math.round(MIN_TILE * SCALES[size]);
}

/** How wide a tile on a rail is, at the size that was chosen. */
export function railTileFor(size: TileSize): number {
  return Math.round(RAIL_TILE * SCALES[size]);
}

/** How many tiles fit across a given width, never fewer than one. */
export function columnsFor(width: number, minimum = MIN_TILE, gap = TILE_GAP): number {
  if (!Number.isFinite(width) || width <= 0) return 1;
  // Every column but the first brings a gap with it, so the gap is added to
  // both sides of the division and taken off the answer.
  return Math.max(1, Math.floor((width + gap) / (minimum + gap)));
}

/** How tall a row is, once the tiles have taken the width between them. */
export function rowHeight(width: number, columns: number, gap = TILE_GAP, caption = 0): number {
  const tile = (width - gap * (columns - 1)) / Math.max(1, columns);
  return Math.round(tile * TILE_RATIO) + caption + gap;
}

/** The items dealt into rows of a given width, in order. */
export function rowsOf<T>(items: readonly T[], columns: number): T[][] {
  const across = Math.max(1, Math.floor(columns));
  const rows: T[][] = [];
  for (let at = 0; at < items.length; at += across) {
    rows.push(items.slice(at, at + across));
  }
  return rows;
}
