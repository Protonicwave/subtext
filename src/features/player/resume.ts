import type { FilmView } from '@/shared/ipc/bindings';

/**
 * Carrying on where you left off.
 *
 * Two decisions, both small enough to be worth stating plainly rather than
 * scattering through the player: where a film starts when you open it again,
 * and when it stops being something to carry on with.
 */

/**
 * The moment a film should open at.
 *
 * A little before where it was left, because coming back in the middle of a
 * line is disorienting. A film that was watched to the end starts again from
 * the beginning: that is what opening it again means.
 */
export function startAtOf(film: FilmView, rewindMs: number): number {
  const position = film.position;
  if (position === null || position.finished) return 0;

  return Math.max(0, position.positionMs - rewindMs);
}

/**
 * Whether this counts as having watched the film.
 *
 * Short of the end, because almost nobody sits through the credits and a film
 * left two minutes from the end should not be offered for ever. A film whose
 * running time is not known yet is never finished, since there is nothing to be
 * near the end of.
 */
export function isFinished(
  positionMs: number,
  durationMs: number | null,
  watchedFraction: number,
): boolean {
  if (durationMs === null || durationMs <= 0) return false;

  return positionMs >= durationMs * watchedFraction;
}
