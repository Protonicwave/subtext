import type { FilmView } from '@/shared/ipc/bindings';

/**
 * What a film says about itself in a line.
 *
 * The year and the running time, which is everything the library holds about a
 * film that is worth putting under its title. A fact that is not known is left
 * out rather than shown empty, since a film nobody has opened has no running
 * time and a file with no year in its name has no year.
 */
export function factsOf(film: FilmView): string | null {
  const parts = [film.year === null ? null : String(film.year), runtimeOf(film.durationMs)].filter(
    (part) => part !== null,
  );

  return parts.length === 0 ? null : parts.join(' · ');
}

/**
 * How long a film runs, said the way somebody choosing what to watch says it.
 *
 * Hours and minutes once there is an hour of it, minutes below that, and never
 * seconds. A film whose running time is not known says nothing at all, since
 * the length is only learnt by opening the file and plenty of films in a
 * library have not been opened yet.
 */
export function runtimeOf(durationMs: number | null): string | null {
  if (durationMs === null || durationMs < 60_000) return null;

  const minutes = Math.round(durationMs / 60_000);
  if (minutes < 60) return `${String(minutes)} min`;

  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `${String(hours)} hr` : `${String(hours)} hr ${String(rest)} min`;
}
