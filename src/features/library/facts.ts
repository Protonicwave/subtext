import type { FilmView } from '@/shared/ipc/bindings';
import { dateOf } from './dates';
import { sizeOf } from './size';

/**
 * What the sheet says a film's file is.
 *
 * All of it was read once, when the film was scanned, and is held on the row
 * from then on: opening the sheet reads nothing. A fact the file did not state
 * is left out rather than shown empty, which is why this returns the rows it
 * has rather than a fixed shape with holes in it. An MP4 is not parsed by
 * Subtext and shows less than a Matroska file does, and showing less is
 * correct.
 */

export interface Fact {
  label: string;
  value: string;
  /** A path, which is cut from the left so the file name is what survives. */
  path?: boolean;
}

export function fileFactsOf(film: FilmView, today = new Date()): Fact[] {
  const details = film.details;
  const video = details?.video ?? null;

  return [
    fact('Container', details?.container),
    fact('Video', video?.codec),
    fact('Resolution', dimensionsOf(film)),
    fact('Bit depth', depthOf(video?.bitDepth ?? null)),
    fact('Frame rate', frameRateOf(video?.frameRate ?? null)),
    fact('Average bitrate', bitrateOf(details?.averageBitrate ?? null)),
    fact('Size', details === null ? null : sizeOf(details.sizeBytes)),
    fact('Added', dateOf(film.addedAt, today)),
    { label: 'Path', value: film.path, path: true },
  ].filter((row) => row !== null);
}

/** A row, or nothing at all where the file did not say. */
function fact(label: string, value: string | null | undefined): Fact | null {
  return value === null || value === undefined || value === '' ? null : { label, value };
}

/**
 * The picture as the container measured it, rather than as it is spoken about.
 *
 * The line under the title already says 1080p, which is how somebody choosing
 * a film talks. This block is the reference, and a reference that repeated the
 * shorthand would be a row that added nothing to the one above it.
 */
function dimensionsOf(film: FilmView): string | null {
  const video = film.details?.video;
  if (!video) return null;

  const { width, height } = video;
  if (width === null || height === null) return null;
  return `${String(width)} × ${String(height)}`;
}

/**
 * Ten bit and eight bit, said the way an encode is described.
 *
 * Worth a row of its own because it is the fact that decides whether a film
 * plays at all on some machines, and it is invisible everywhere else.
 */
function depthOf(bits: number | null): string | null {
  return bits === null ? null : `${String(bits)}-bit`;
}

/**
 * Frames a second, to three places.
 *
 * The films in circulation are 23.976 and 29.97 as often as they are whole
 * numbers, and rounding those to 24 and 30 would lose the distinction the
 * framerate conversions in the timing panel exist to correct.
 */
function frameRateOf(rate: number | null): string | null {
  if (rate === null || !Number.isFinite(rate) || rate <= 0) return null;

  // Whole rates as whole numbers, since 25.000 fps is a figure nobody writes.
  const written = Number.isInteger(rate) ? String(rate) : rate.toFixed(3);
  return `${written} fps`;
}

/**
 * How much of the film arrives each second, on average across the whole of it.
 *
 * Labelled as an average where it is shown, because no encode of anything worth
 * watching holds one rate from beginning to end.
 */
function bitrateOf(bitsPerSecond: number | null): string | null {
  if (bitsPerSecond === null || bitsPerSecond <= 0) return null;

  if (bitsPerSecond >= 1_000_000) return `${(bitsPerSecond / 1_000_000).toFixed(1)} Mb/s`;
  return `${String(Math.round(bitsPerSecond / 1_000))} kb/s`;
}
