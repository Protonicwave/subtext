import type { FilmView } from '@/shared/ipc/bindings';

/**
 * How much of a film is left, said the way somebody deciding what to watch
 * would say it.
 *
 * Minutes, and hours once there are enough of them to be worth grouping. Never
 * seconds: nobody choosing between two films cares about seconds, and a figure
 * that changes while you read it is noise.
 */
export function remainingOf(film: FilmView): string {
  const left = leftMsOf(film);
  if (left === null) return 'Carry on';
  // Under a minute is the end of the film in every practical sense, and
  // "0 min left" reads as an error.
  if (left < FINISHING_MS) return 'Nearly finished';

  const minutes = Math.round(left / 60_000);
  if (minutes < 60) return `${String(minutes)} min left`;

  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `${String(hours)} hr left` : `${String(hours)} hr ${String(rest)} min left`;
}

/**
 * How near the end counts as finished.
 *
 * Written down rather than repeated, since the masthead has to know it too: a
 * film this near the end is described rather than measured, and a description
 * takes nothing after it.
 */
export const FINISHING_MS = 60_000;

/**
 * How much of a film is left in milliseconds, where that is known at all.
 *
 * The running time comes from the saved position first and from the film
 * second, because a film whose file has not been opened by this build still
 * has whatever length was recorded the last time somebody watched it.
 */
export function leftMsOf(film: FilmView): number | null {
  const position = film.position;
  const duration = position?.durationMs ?? film.durationMs;
  if (!position || duration === null) return null;

  return Math.max(0, duration - position.positionMs);
}
