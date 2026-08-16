import type { FilmView } from '@/shared/ipc/bindings';

/**
 * Finding a film by what it is called and where it is filed.
 *
 * A substring test over an array prepared once when the library loads, rather
 * than an index. Ten thousand films is ten thousand short strings, and walking
 * them costs less than a millisecond, which is under the keystroke this has to
 * answer inside. An index would have to be built at the same cost, kept in step
 * with every scan that changes a row, and would still come back to the same
 * answer for a query of one letter.
 *
 * Folding happens once, on the way into the array, so a keystroke folds the
 * query alone. Accents are folded as well as case, because somebody typing
 * amelie is looking for Amélie and a library holds the films of more than one
 * language whatever keyboard is in front of it.
 */

/** A film, and everything it can be found by, folded once. */
export interface Entry {
  readonly film: FilmView;
  readonly text: string;
}

/** What was found, and how much of it the palette is not drawing. */
export interface Found {
  readonly films: readonly FilmView[];
  /** Matched but not returned. Zero where everything matched is here. */
  readonly withheld: number;
}

/**
 * How many films are drawn before the list says how many more there are.
 *
 * A palette is for finding one film rather than for reading a library, and a
 * query that matches four hundred is a query that wants another letter. The
 * count is shown rather than the rest being dropped quietly.
 */
export const SHOWN = 40;

/**
 * Text as it is compared: lower case, and without the marks over the letters.
 *
 * Decomposing first is what separates a mark from the letter it sits on, so
 * that removing every mark leaves the letter behind rather than the character
 * whole.
 */
export function foldedOf(text: string): string {
  return text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
}

/**
 * The library as the palette searches it.
 *
 * A film is found by its title and by the folder it sits in, which is the
 * shelf the library screen draws it on and the answer to "it is in with the
 * westerns somewhere". The two are joined by a newline so that a query cannot
 * match across the join and turn the end of a title and the start of a folder
 * into a word neither of them contains.
 */
export function entriesOf(films: readonly FilmView[]): Entry[] {
  return films.map((film) => ({ film, text: foldedOf(`${film.title}\n${film.shelf.name}`) }));
}

/**
 * The films a query matches, in the order the library holds them.
 *
 * Nothing at all for a query of spaces: the palette opens on the actions, and a
 * blank field offering ten thousand films in no particular order would be a
 * list nobody asked for.
 */
export function filmsMatching(
  entries: readonly Entry[],
  query: string,
  shown: number = SHOWN,
): Found {
  const wanted = foldedOf(query.trim());
  if (wanted === '') return { films: [], withheld: 0 };

  const films: FilmView[] = [];
  let withheld = 0;

  for (const entry of entries) {
    if (!entry.text.includes(wanted)) continue;
    if (films.length < shown) films.push(entry.film);
    else withheld += 1;
  }

  return { films, withheld };
}

/**
 * Where the selection lands after a move.
 *
 * Wraps at both ends, because a list reached by the down arrow is a list whose
 * last item is one key from the first. An empty list has nothing to select and
 * stays at nothing.
 */
export function movedBy(selected: number, by: number, length: number): number {
  if (length === 0) return 0;

  return (((selected + by) % length) + length) % length;
}
