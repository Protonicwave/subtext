import type { CueView } from '@/shared/ipc/bindings';

/**
 * Where to watch, to see whether a measurement landed.
 *
 * An alignment reports what it did and what the film said about it, and both of
 * those are numbers. The only thing that settles the question is the film
 * itself, so the last thing a measurement does is offer to show it: a stretch
 * where people are talking over each other, played for a few seconds, where a
 * subtitle that is out by a quarter of a second is obvious and one that is right
 * is equally obvious.
 *
 * Which stretch is a property of the dialogue and of nothing else, so it is
 * worked out here rather than in either of the two screens that offer the check.
 */

/**
 * How much film a check is judged over, in milliseconds.
 *
 * Twenty seconds, which is the stretch density is measured across and about as
 * long as anybody needs to see whether the lines are arriving with the voices.
 * Nothing stops at the end of it: the film goes on playing and the question
 * waits, because a film that paused itself to ask would be answering a
 * different question than the one about watching it.
 */
export const WATCH_MS = 20_000;

/**
 * How far before the first line the film is put, in milliseconds.
 *
 * A second, so that the line is seen to arrive rather than being on screen
 * already. A line that is up before the voice cannot be told from one that is
 * late by the same amount, and arriving is the whole thing being judged.
 */
export const LEAD_MS = 1_000;

/**
 * The moment to watch, or nothing where the track has no dialogue in it.
 *
 * The busiest stretch there is: the window of [`WATCH_MS`] holding the most
 * lines, which is a scene of people talking rather than a lone line over a
 * landscape. Density is what makes a check quick to make. One line every ten
 * seconds takes a minute to judge and several lines in a row take one of them.
 *
 * Ties go to the earliest, which is nearer wherever somebody already is in the
 * film and no worse on its merits.
 *
 * The cues are taken as the player would draw them, which means with whatever
 * correction is in force already applied. What comes back is therefore a
 * position in the film rather than a position in the file.
 */
export function busiestMomentOf(cues: readonly CueView[]): number | null {
  if (cues.length === 0) return null;

  // One walk, with the end of the window running ahead of the start of it and
  // never going back, because the cues are in playback order.
  let end = 0;
  let busiest = 0;
  let most = 0;

  for (let start = 0; start < cues.length; start += 1) {
    const from = cues[start]?.startMs ?? 0;
    if (end < start) end = start;
    while (end < cues.length && (cues[end]?.startMs ?? 0) < from + WATCH_MS) end += 1;

    const lines = end - start;
    if (lines > most) {
      most = lines;
      busiest = start;
    }
  }

  return Math.max(0, (cues[busiest]?.startMs ?? 0) - LEAD_MS);
}
