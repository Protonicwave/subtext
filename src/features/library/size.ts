import type { FilmView } from '@/shared/ipc/bindings';

/**
 * How large something is, said the way a drive says it.
 *
 * Decimal rather than binary, because that is what the boxes, the shops and
 * every file manager a person has seen use, and a library that reads as 401 GB
 * everywhere else should not read as 373 here.
 */
export function sizeOf(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  if (bytes >= 1_000_000) return `${String(Math.round(bytes / 1_000_000))} MB`;
  if (bytes >= 1_000) return `${String(Math.round(bytes / 1_000))} kB`;

  return `${String(Math.max(0, Math.round(bytes)))} bytes`;
}

/**
 * How much of the disk a library takes, or nothing where that is not known.
 *
 * A film recorded before this build described such things has no size until the
 * next read of its folder, and a total that quietly left those out would be
 * wrong without saying so. All of it or none of it.
 */
export function libraryTotalOf(films: readonly FilmView[]): number | null {
  let total = 0;
  for (const film of films) {
    if (!film.details) return null;
    total += film.details.sizeBytes;
  }

  return films.length === 0 ? null : total;
}
