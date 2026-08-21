import type { CueView } from '@/shared/ipc/bindings';

/**
 * Where a film does its talking.
 *
 * A scrubber that is a plain line says nothing about the film behind it. The
 * same line with the dialogue drawn under it says where the exchanges are, where
 * the long quiet stretch is, and where the film stops speaking for the last
 * twenty minutes. Somebody dragging the bar is looking for a scene, and a scene
 * is easier to find by the shape of its dialogue than by a timecode.
 *
 * It costs one pass over cues that are already in memory for the subtitles, so
 * this adds no read, no table and no command. It is worked out once for a track
 * and not again while the film runs.
 *
 * The shape is buckets rather than cues, because a film has thousands of cues
 * and a scrubber is a thousand pixels wide at its widest. Each bucket holds how
 * much of its stretch of the film was spoken over, and the busiest bucket in the
 * film reaches the full height, so a talkative film and a quiet one are each
 * drawn against themselves.
 */

/**
 * How many buckets the running time is cut into.
 *
 * A few pixels each at any window width. More would draw detail that lands
 * inside one pixel; many fewer and an exchange of a dozen lines becomes a
 * single block.
 */
export const BUCKETS = 320;

/**
 * How much of each bucket was spoken over, from zero to one.
 *
 * A film whose length is not known yet has no buckets to fill: the running time
 * is what says where a cue falls, and there is nothing to draw without it.
 */
export function talkingOf(
  cues: readonly CueView[],
  durationMs: number | null,
  buckets = BUCKETS,
): number[] {
  if (durationMs === null || durationMs <= 0 || buckets <= 0) return [];

  const held = new Array<number>(buckets).fill(0);
  const each = durationMs / buckets;

  for (const cue of cues) {
    const from = bucket(cue.startMs, each, buckets);
    // A cue covers its start and not its end, so a cue ending exactly on a
    // boundary belongs to the bucket before it. A cue of no length covers
    // nothing and is drawn nowhere, which is what the player does with it too.
    const to = bucket(cue.endMs - 1, each, buckets);

    for (let at = from; at <= to; at += 1) {
      // How much of this bucket the cue covers, clamped to the film at both
      // ends so that a track running past the last frame piles into the last
      // bucket instead of reaching past it.
      const spoken =
        Math.min(cue.endMs, durationMs, (at + 1) * each) - Math.max(cue.startMs, at * each);
      if (spoken > 0) held[at] = (held[at] ?? 0) + spoken;
    }
  }

  // Against the busiest bucket rather than against the length of a bucket,
  // because lines that overlap can fill one more than once over and a film of
  // short exchanges would otherwise be drawn as a flat nothing.
  const most = held.reduce((highest, filled) => Math.max(highest, filled), 0);
  return most <= 0 ? held : held.map((filled) => filled / most);
}

/** How tall the busiest bucket stands, in the shape's own units. */
const REACH = 100;

/**
 * The shape, as one path hanging from the top of its box.
 *
 * One path rather than one element a bucket, because three hundred elements
 * under a control bar is three hundred elements to lay out every time the bar
 * appears. It is a fill under the scrubber's own line and not a chart: there is
 * no floor drawn under the quiet stretches, since the bar above it is already
 * the line somebody drags.
 */
export function shapeOf(talking: readonly number[]): string {
  if (talking.length === 0) return '';

  const tops = talking.map((filled, at) => {
    // The middle of the bucket: the shape is a line through the buckets rather
    // than a row of columns, which at a few pixels each reads as texture.
    const x = round(at + 0.5);
    return `${x},${round(Math.max(0, Math.min(1, filled)) * REACH)}`;
  });

  return `M0,0L${tops.join('L')}L${String(talking.length)},0Z`;
}

/** A coordinate, kept short: this string is a few hundred of them. */
function round(value: number): string {
  return String(Math.round(value * 10) / 10);
}

/** Which bucket a moment falls in, never outside the film. */
function bucket(ms: number, each: number, buckets: number): number {
  return Math.max(0, Math.min(buckets - 1, Math.floor(ms / each)));
}
