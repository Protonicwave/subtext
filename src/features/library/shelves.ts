import type { FilmView } from '@/shared/ipc/bindings';

/**
 * The rows the library screen is made of.
 *
 * One row per folder somebody made, which is the whole idea: people already
 * sorted their films, and reading that sort costs nothing and is right more
 * often than any taxonomy this application could invent. Which folder a film
 * belongs to was decided in Rust and arrives on the film.
 *
 * One computed row sits above them, for what there is to carry on with, since
 * that is the question the screen is most often opened to answer.
 */

/** The row above the folders, which belongs to no folder. */
export const CARRYING_ON = 'carrying-on';

export interface Shelf {
  /** What tells one row from another, and what React keys them by. */
  key: string;
  name: string;
  /** The folder on disk, or nothing for the computed row. */
  path: string | null;
  films: FilmView[];
}

/**
 * The films arranged into rows, the computed one first.
 *
 * Folders are in the order they were first met on disk, which is the order
 * their films were first recorded and so is the order of the lowest identifier
 * on each row. That is stable between runs, unlike anything derived from the
 * library's own ordering, which is alphabetical and would put a row named Zulu
 * above one named Alice depending on what happened to be in them.
 */
export function shelvesOf(films: readonly FilmView[], resumable: readonly FilmView[]): Shelf[] {
  const byFolder = new Map<string, Shelf>();
  const met = new Map<string, number>();

  for (const film of films) {
    const key = film.shelf.path;
    const shelf = byFolder.get(key);

    if (shelf) {
      shelf.films.push(film);
      met.set(key, Math.min(met.get(key) ?? film.id, film.id));
    } else {
      byFolder.set(key, { key, name: film.shelf.name, path: key, films: [film] });
      met.set(key, film.id);
    }
  }

  const shelves = [...byFolder.values()].sort(
    (one, other) => (met.get(one.key) ?? 0) - (met.get(other.key) ?? 0),
  );

  // Already in the order they were stopped at, most recent first, which is the
  // order this row is for.
  if (resumable.length > 0) {
    shelves.unshift({
      key: CARRYING_ON,
      name: 'Carry on watching',
      path: null,
      films: [...resumable],
    });
  }

  return shelves;
}

/**
 * The one film shown large above the rows.
 *
 * What was stopped most recently, because somebody who left a film part way
 * through is more likely to have come back for it than for anything else. With
 * nothing part watched it is the most recently added film, which is the newest
 * identifier: a library nobody has watched yet still has something to show, and
 * what arrived last is the likeliest reason the application was opened.
 *
 * A film whose file has gone is never chosen. The billboard offers to play it,
 * and offering to play a file that is not there is the one thing this screen
 * should not do.
 */
export function billboardOf(
  films: readonly FilmView[],
  resumable: readonly FilmView[],
): FilmView | null {
  const stopped = resumable.find((film) => !film.missing);
  if (stopped) return stopped;

  let newest: FilmView | null = null;
  for (const film of films) {
    if (film.missing) continue;
    if (newest === null || film.id > newest.id) newest = film;
  }

  return newest;
}
