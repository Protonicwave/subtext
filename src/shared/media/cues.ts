import type { CueView } from '@/shared/ipc/bindings';

/**
 * Which line of dialogue is on screen, and which ones are either side of it.
 *
 * Every question here is answered against the same sorted array by the same
 * binary search, which is what lets the subtitles and the
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
   * screen, and the walk stops.
   *
   * Measured after the reading comfort adjustment rather than before it, since
   * both preferences make cues longer. A bound taken from the file as written
   * would end the walk back early and drop a line that is still up.
   */
  readonly longest: number;
}

/**
 * How long a line is given to be read, over and above what the file says.
 *
 * Broadcast practice puts a line up slightly before it is spoken and holds it
 * for a minimum however short it is, because reading is slower than hearing. A
 * cue that begins on the first syllable is right and still reads as late. Most
 * amateur files snap to the syllable, so this is applied to all of them.
 *
 * It says nothing about the cues themselves, which is why it is a preference
 * applied here rather than a correction stored beside them.
 */
export interface Comfort {
  /**
   * How far before its written start a line appears.
   *
   * The start only. Holding a line for longer is the convention; moving it
   * wholesale would put the end of it before the end of the sentence.
   */
  readonly leadInMs: number;
  /** The shortest time any line stays on screen, reached by extending the end. */
  readonly minimumMs: number;
}

/** The file honoured exactly, which is what a lookup gets when nobody says otherwise. */
export const AS_WRITTEN: Comfort = { leadInMs: 0, minimumMs: 0 };

/**
 * The timeline the player reads, with everything that changes a timing applied.
 *
 * `shiftMs` moves the whole track, and is how the sync control shows what it is
 * doing while somebody is still arriving at a value. It is not where a
 * correction lives: a correction is a property of the track, it is stored on
 * its row and applied in Rust as the cues are read, and the cues handed to this
 * function have already been through it. This is the provisional half, the
 * difference between what is being tried and what has been written down, and it
 * goes to zero the moment the value is committed and the track is read again.
 */
export function timelineOf(
  cues: readonly CueView[],
  comfort: Comfort = AS_WRITTEN,
  shiftMs = 0,
): Timeline {
  const read = readable(cues, comfort, shiftMs);

  let longest = 0;
  for (const cue of read) {
    const length = cue.endMs - cue.startMs;
    if (length > longest) longest = length;
  }
  return { cues: read, longest };
}

/**
 * The cues as they are to be drawn, in one pass over them.
 *
 * Neither preference may put two lines on screen that were not there already:
 * a line is led in no further than the end of the line before it, and extended
 * no further than the start of the line after it. Where a file overlaps its own
 * cues those overlaps are left as they are, since the file meant them and this
 * is not the place to argue.
 */
function readable(
  cues: readonly CueView[],
  { leadInMs, minimumMs }: Comfort,
  shiftMs: number,
): readonly CueView[] {
  // Nothing asked for, so nothing copied. Worth the line: it is the whole cost
  // of the feature for anybody who has turned all of them off, which is every
  // film nobody is adjusting.
  if (leadInMs <= 0 && minimumMs <= 0 && shiftMs === 0) return cues;

  const read: CueView[] = [];
  // The end of the line before this one, moved but not led in. What the
  // lead-in of this one is not allowed past.
  let after = 0;

  for (let at = 0; at < cues.length; at += 1) {
    const cue = cues[at];
    if (cue === undefined) continue;

    // The shift first, since it says where the line is and the two preferences
    // say how long it is given once it is there.
    const moved = movedTo(cue.startMs, shiftMs);
    const ends = movedTo(cue.endMs, shiftMs);

    const startMs = shownFrom(moved, after, leadInMs);
    // Where the next line will appear, worked out the same way it will be when
    // its own turn comes. This is what the extension stops at.
    const next = cues[at + 1];
    const until =
      next === undefined
        ? Number.POSITIVE_INFINITY
        : shownFrom(movedTo(next.startMs, shiftMs), ends, leadInMs);

    read.push({
      ...cue,
      startMs,
      // Never shortened: a cue already running past the next one is a file that
      // overlaps, and taking time off it would be a change nobody asked for.
      endMs: Math.max(ends, Math.min(startMs + minimumMs, until)),
    });

    after = ends;
  }

  return read;
}

/** Where a moment lands once the whole track has been moved, never before zero. */
function movedTo(ms: number, shiftMs: number): number {
  return Math.max(0, ms + shiftMs);
}

/** When a line appears, given when the one before it finished. */
function shownFrom(startMs: number, afterMs: number, leadInMs: number): number {
  // Not before the film begins, and not back over the line before. Where that
  // line already runs past this one the floor is this line's own start, which
  // leaves it where the file put it.
  const floor = Math.max(0, Math.min(afterMs, startMs));
  return Math.max(startMs - leadInMs, floor);
}

/**
 * The index of the line on screen at a moment, or [`NONE`].
 *
 * A cue covers its start and not its end, which matches how the parser reads
 * them: a cue of no length is never drawn, though it is still part of the
 * file.
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
