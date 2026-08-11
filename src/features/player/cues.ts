import type { CueView } from '@/shared/ipc/bindings';

/**
 * Which line of dialogue is on screen.
 *
 * This runs on every animation frame for the whole length of a film, so it is a
 * binary search over a sorted array and not a scan. Across five thousand cues
 * the difference is thirteen comparisons against two and a half thousand, sixty
 * times a second, and it is the reason the cues are held in memory as one array
 * rather than asked for a window at a time.
 *
 * Nothing here allocates. It is called from the frame loop, and a function that
 * makes so much as an object per call makes the garbage collector part of the
 * playback path.
 */

/** No line is on screen, which is most of a film. */
export const NONE = -1;

/**
 * The cues of one track, with the one fact a lookup needs that the array itself
 * does not carry.
 */
export interface Timeline {
  readonly cues: readonly CueView[];
  /**
   * The length of the longest cue, in milliseconds.
   *
   * Cues are sorted by when they start, not by when they end, so a cue that
   * starts earlier can still be on screen after a later one has begun. This is
   * what bounds the walk back through the overlap: once a cue started longer
   * ago than the longest cue lasts, neither it nor anything before it can be on
   * screen, and the search stops.
   */
  readonly longest: number;
}

export function timelineOf(cues: readonly CueView[]): Timeline {
  let longest = 0;
  for (const cue of cues) {
    const length = cue.endMs - cue.startMs;
    if (length > longest) longest = length;
  }
  return { cues, longest };
}

/**
 * The index of the line on screen at a moment, or [`NONE`].
 *
 * A cue covers its start and not its end, which matches how the parser reads
 * them: a cue of no length is never drawn, though its text is still searchable.
 * Where two cues overlap the later one wins, since that is the line that has
 * just been spoken.
 */
export function activeAt({ cues, longest }: Timeline, ms: number): number {
  let low = 0;
  let high = cues.length - 1;
  let found = NONE;

  // The last cue that had started by now. Everything on screen begins at or
  // before this one.
  while (low <= high) {
    const middle = (low + high) >> 1;
    if ((cues[middle]?.startMs ?? 0) <= ms) {
      found = middle;
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }

  for (let at = found; at >= 0; at -= 1) {
    const cue = cues[at];
    if (cue === undefined) break;
    // Far enough back that no cue from here on can still be running.
    if (cue.startMs + longest <= ms) break;
    if (ms < cue.endMs) return at;
  }

  return NONE;
}
