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
  const position = film.position;
  const duration = position?.durationMs ?? film.durationMs;
  if (!position || duration === null) return 'Carry on';

  const left = duration - position.positionMs;
  // Under a minute is the end of the film in every practical sense, and
  // "0 min left" reads as an error.
  if (left < 60_000) return 'Nearly finished';

  const minutes = Math.round(left / 60_000);
  if (minutes < 60) return `${String(minutes)} min left`;

  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `${String(hours)} hr left` : `${String(hours)} hr ${String(rest)} min left`;
}
