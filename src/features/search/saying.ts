import type { SearchView } from '@/shared/ipc/bindings';

/**
 * What the palette says about what it found, and about what it did not.
 *
 * Kept apart from the component because these are the sentences, and sentences
 * are worth reading in one place and testing without a screen. The rule behind
 * all of them: say what was searched and how much of it, rather than "no
 * results". A search that finds nothing in a library nobody has indexed is a
 * different thing from a search that finds nothing in a million indexed lines,
 * and only one of them means the word is not said.
 */

/** How much there is to search, on one side of the boundary or the other. */
export interface Corpus {
  films: number;
  lines: number;
}

/** The one film a search is being kept to. */
export interface Scope {
  title: string;
  lines: number;
}

function counted(value: number): string {
  return value.toLocaleString('en-GB');
}

function plural(value: number, one: string, many: string): string {
  return `${counted(value)} ${value === 1 ? one : many}`;
}

/** What is on the screen, said as a count. */
export function foundSaying(results: SearchView): string {
  const films = results.films.length;
  const lines = plural(results.shown, 'line', 'lines');

  // Whether these are the best matches or simply the first ones is the sort of
  // thing an application usually keeps to itself. It is worth a clause: without
  // it, a search whose best match is on the second screen looks like a search
  // that ranked badly.
  const order = results.ranked ? '' : ', in the order they are said';
  const first = results.truncated ? 'the first ' : '';

  if (films === 1) return `${first}${lines}${order}`;
  return `${first}${lines} in ${plural(films, 'film', 'films')}${order}`;
}

/** Why a search came back with nothing, in terms of what there was to search. */
export function nothingSaying(query: string, scope: Scope | null, library: Corpus): string {
  const quoted = `“${query.trim()}”`;

  if (scope !== null) {
    if (scope.lines === 0) {
      return `${scope.title} has no subtitle file paired with it, so there is nothing of it to search. Widening the search looks through the rest of your films.`;
    }
    return `Nothing in the ${plural(scope.lines, 'line', 'lines')} of ${scope.title} matches ${quoted}. Widening the search looks through the rest of your films.`;
  }

  if (library.lines === 0) {
    // Not a failed search at all. Search reads the subtitle files paired with
    // films, and this library has none, which is worth saying rather than
    // leaving somebody to conclude that nobody in their films says anything.
    return library.films === 0
      ? 'There are no films to search yet. Adding a folder is what puts their dialogue here.'
      : 'None of your films has a subtitle file paired with it yet, so there is no dialogue to search.';
  }

  return `Nothing in the ${plural(library.lines, 'line', 'lines')} of your ${plural(library.films, 'film', 'films')} matches ${quoted}.`;
}
