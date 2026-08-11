import type { CueView } from '@/shared/ipc/bindings';

/**
 * How much is said, and where.
 *
 * A scrubber that is a plain line says nothing about the film behind it. The
 * same line drawn from the dialogue says a great deal: the act breaks are the
 * valleys, the long quiet stretch two thirds of the way in is the one everybody
 * remembers, and the chase near the end is the flat part where nobody speaks.
 * It costs one pass over cues that are already in memory.
 *
 * The shape is bands rather than cues, because a film has thousands of cues and
 * a scrubber is eight hundred pixels wide. Each band holds how much of its
 * stretch of the film was spoken over, and the busiest band in the film is the
 * full height, so that a talkative film and a quiet one are each drawn against
 * themselves.
 */

/**
 * How many bands the film is cut into.
 *
 * A few per pixel of a scrubber at its widest. More would draw detail nobody
 * can see; many fewer and an exchange of a dozen lines becomes one block.
 */
export const BANDS = 240;

/** How much of each band was spoken over, from zero to one. */
export function densityOf(
  cues: readonly CueView[],
  durationMs: number | null,
  bands = BANDS,
): number[] {
  const map = new Array<number>(bands).fill(0);
  if (durationMs === null || durationMs <= 0) return map;

  const each = durationMs / bands;
  for (const cue of cues) {
    const from = band(cue.startMs, each, bands);
    // A cue covers its start and not its end, so a cue ending exactly on a
    // boundary belongs to the band before it.
    const to = band(cue.endMs - 1, each, bands);

    for (let at = from; at <= to; at += 1) {
      const spoken = Math.min(cue.endMs, (at + 1) * each) - Math.max(cue.startMs, at * each);
      if (spoken > 0) map[at] = (map[at] ?? 0) + spoken;
    }
  }

  // Against the busiest band rather than against the length of a band, because
  // lines that overlap can fill one more than once over and a film of quiet
  // exchanges would otherwise be drawn as a flat line.
  const most = map.reduce((highest, held) => Math.max(highest, held), 0);
  return most <= 0 ? map : map.map((held) => held / most);
}

function band(ms: number, each: number, bands: number): number {
  return Math.max(0, Math.min(bands - 1, Math.floor(ms / each)));
}

/** Where the middle of the drawn shape sits, in the shape's own units. */
const MIDDLE = 50;

/** How far the busiest band reaches from the middle. */
const REACH = 44;

/**
 * How far a silent stretch reaches.
 *
 * Not nothing: this is the scrubber's own track as well as a picture of the
 * film, and a track with gaps in it is one nobody would think to drag.
 */
const QUIET = 3;

/**
 * The shape, as a path drawn about the middle of the bar.
 *
 * Mirrored rather than standing on the floor of the bar, which reads as a
 * thickening and thinning line rather than as a chart, and leaves the thumb
 * sitting on the middle of it wherever it is.
 */
export function shapeOf(density: readonly number[]): string {
  if (density.length === 0) return '';

  const above: string[] = [];
  const below: string[] = [];

  density.forEach((held, at) => {
    const half = QUIET + Math.max(0, Math.min(1, held)) * (REACH - QUIET);
    // The middle of the band: the shape is a line through the bands and not a
    // row of columns, at a few pixels each.
    const x = round(at + 0.5);
    above.push(`${x},${round(MIDDLE - half)}`);
    below.push(`${x},${round(MIDDLE + half)}`);
  });

  return `M${above.join('L')}L${below.reverse().join('L')}Z`;
}

/** A coordinate, kept short: this string is a few hundred of them. */
function round(value: number): string {
  return String(Math.round(value * 10) / 10);
}

/**
 * The shapes worked out so far, so that a film opened again is not measured
 * again. Kept to a handful: each is a few hundred numbers, and the film being
 * watched is almost always the one just before it in this list.
 */
const remembered = new Map<string, string>();

/** How many films are kept. */
const KEEP = 8;

/** The shape for a film, worked out once. */
export function shapeFor(key: string, cues: readonly CueView[], durationMs: number | null): string {
  const held = remembered.get(key);
  if (held !== undefined) return held;

  const shape = shapeOf(densityOf(cues, durationMs));
  remembered.set(key, shape);

  const oldest = remembered.keys().next();
  if (remembered.size > KEEP && !oldest.done) remembered.delete(oldest.value);

  return shape;
}
