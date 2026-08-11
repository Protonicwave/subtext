import type { CueView } from '@/shared/ipc/bindings';

/**
 * Which line of dialogue is on screen, and which ones are either side of it.
 *
 * Every question here is answered against the same sorted array by the same
 * binary search, which is what lets the transcript, the subtitles and the
 * scrubber all ask about a moment without any of them holding a second copy of
 * the dialogue or an index of their own.
 *
 * The lookup runs on every animation frame for the whole length of a film, so it is a
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
export function activeAt(timeline: Timeline, ms: number): number {
  const { cues, longest } = timeline;
  // The last cue that had started by now. Everything on screen begins at or
  // before this one.
  const found = startedBy(timeline, ms);

  for (let at = found; at >= 0; at -= 1) {
    const cue = cues[at];
    if (cue === undefined) break;
    // Far enough back that no cue from here on can still be running.
    if (cue.startMs + longest <= ms) break;
    if (ms < cue.endMs) return at;
  }

  return NONE;
}

/**
 * The index of the last cue that had started by a moment, or [`NONE`].
 *
 * Where more than one begins at the same moment it is the last of them, which
 * is what makes the cue after it the next one to be spoken rather than one of
 * the same breath.
 */
function startedBy({ cues }: Timeline, ms: number): number {
  let low = 0;
  let high = cues.length - 1;
  let found = NONE;

  while (low <= high) {
    const middle = (low + high) >> 1;
    if ((cues[middle]?.startMs ?? 0) <= ms) {
      found = middle;
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }

  return found;
}

/**
 * How far into a line going back means going back to its start.
 *
 * Without it, going back from the start of a line would land on the line it is
 * already on and pressing it twice would do nothing the second time. With it,
 * going back part way through a line replays that line, which is what somebody
 * who missed what was said meant, and going back again steps behind it.
 */
const LEAD = 400;

/** The line after a moment, or [`NONE`] when the dialogue has run out. */
export function nextFrom(timeline: Timeline, ms: number): number {
  const after = startedBy(timeline, ms) + 1;
  return after < timeline.cues.length ? after : NONE;
}

/** The line to go back to from a moment, or [`NONE`] before the first one. */
export function previousFrom(timeline: Timeline, ms: number): number {
  return startedBy(timeline, ms - LEAD);
}

/**
 * The line spoken around a moment, or [`NONE`] if the film is quiet there.
 *
 * What the scrubber shows under the pointer, where a moment landing in a
 * silence should still say what is being said around it rather than nothing.
 * The line on screen wins; failing that, whichever of the lines either side is
 * closer, so long as it is close enough to be the same scene.
 */
export function nearestFrom(timeline: Timeline, ms: number, withinMs: number): number {
  const active = activeAt(timeline, ms);
  if (active !== NONE) return active;

  const before = startedBy(timeline, ms);
  const after = before + 1;

  // How far the moment sits outside each of them. A cue that has finished is
  // measured from its end, and one that has not begun from its start.
  const behind = gapTo(timeline, before, ms);
  const ahead = gapTo(timeline, after, ms);

  const closest = behind <= ahead ? before : after;
  return Math.min(behind, ahead) <= withinMs ? closest : NONE;
}

/** How long a moment falls outside a cue, or forever if there is no such cue. */
function gapTo({ cues }: Timeline, at: number, ms: number): number {
  const cue = cues[at];
  if (cue === undefined) return Number.POSITIVE_INFINITY;
  return ms < cue.startMs ? cue.startMs - ms : ms - cue.endMs;
}
