import type { Id, SearchView, SnippetPart } from '@/shared/ipc/bindings';

/**
 * Search results as one list rather than as a list of lists.
 *
 * The results are grouped by film and drawn that way, but they are moved
 * through by the arrow keys one line at a time, across the group boundaries
 * without stopping on them. Nested lists would mean the selection was a pair of
 * numbers and every move a piece of arithmetic about where a group ends. Flat,
 * it is one index, and which lines start a group is something each row carries.
 */

/** One line of dialogue, and where it sits among the rest. */
export interface ResultRow {
  readonly filmId: Id;
  readonly title: string;
  readonly year: number | null;
  readonly startMs: number;
  readonly snippet: readonly SnippetPart[];
  /** The first line of its film, which is the row that draws the heading. */
  readonly opensFilm: boolean;
  /**
   * Lines in this film that were not returned, counted on its last row. Zero
   * everywhere else, so only one row per film has anything to say about it.
   */
  readonly withheld: number;
}

export function rowsOf(results: SearchView | null): ResultRow[] {
  if (results === null) return [];

  return results.films.flatMap((film) =>
    film.hits.map((hit, at) => ({
      filmId: film.filmId,
      title: film.title,
      year: film.year,
      startMs: hit.startMs,
      snippet: hit.snippet,
      opensFilm: at === 0,
      withheld: at === film.hits.length - 1 ? film.withheld : 0,
    })),
  );
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
