import { describe, expect, it } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { SHOWN, entriesOf, filmsMatching, foldedOf, movedBy } from './matching';

/** A film, with only the two things the palette looks at worth naming. */
function film(id: number, title: string, shelf = 'Films'): FilmView {
  return {
    id,
    folderId: 1,
    path: `/films/${shelf}/${title}.mkv`,
    shelf: { name: shelf, path: `/films/${shelf}` },
    title,
    year: 1995,
    addedAt: 0,
    durationMs: null,
    posterPath: null,
    accent: null,
    missing: false,
    details: null,
    tracks: [],
    chosenTrackId: null,
    subtitlesOff: false,
    position: null,
  };
}

/** The titles a query finds, which is what every test here is about. */
function titlesFor(films: readonly FilmView[], query: string): string[] {
  return filmsMatching(entriesOf(films), query).films.map((found) => found.title);
}

describe('folding', () => {
  it('ignores case', () => {
    expect(foldedOf('The Third Man')).toBe('the third man');
  });

  /*
   * Somebody typing amelie is looking for Amélie. A library holds the films of
   * more than one language whatever keyboard is in front of it, and a title
   * that can only be found by producing its accents is a title that cannot be
   * found.
   */
  it('ignores the marks over the letters', () => {
    expect(foldedOf('Amélie')).toBe('amelie');
    expect(foldedOf('La Haine · Les Diaboliques')).toBe('la haine · les diaboliques');
  });

  it('leaves a letter that is not a mark alone', () => {
    expect(foldedOf('Straße')).toBe('straße');
  });
});

describe('finding a film', () => {
  const library = [
    film(1, 'Heat', 'Crime'),
    film(2, 'The Third Man', 'Noir'),
    film(3, 'Amélie', 'French'),
    film(4, 'Chungking Express', 'Wong Kar-wai'),
  ];

  it('matches part of a title, anywhere in it', () => {
    expect(titlesFor(library, 'third')).toEqual(['The Third Man']);
    expect(titlesFor(library, 'express')).toEqual(['Chungking Express']);
  });

  it('matches whatever accents were typed, and whatever were not', () => {
    expect(titlesFor(library, 'amelie')).toEqual(['Amélie']);
    expect(titlesFor(library, 'Amélie')).toEqual(['Amélie']);
  });

  /*
   * The folder is the shelf the library screen files a film under, and it is
   * how somebody who cannot remember a title but knows where they put it finds
   * the film again.
   */
  it('matches the folder the film is filed in', () => {
    expect(titlesFor(library, 'noir')).toEqual(['The Third Man']);
    expect(titlesFor(library, 'wong')).toEqual(['Chungking Express']);
  });

  /*
   * The title and the folder are two facts, not one string. Matching across
   * the join would find a film by a word that appears nowhere in either, which
   * is the sort of result that reads as a bug in the matching.
   */
  it('does not match across the join between the two', () => {
    expect(titlesFor([film(1, 'Heat', 'Crime')], 'heatcrime')).toEqual([]);
    expect(titlesFor([film(1, 'Heat', 'Crime')], 'heat crime')).toEqual([]);
  });

  it('keeps the order the library holds them in', () => {
    expect(titlesFor(library, 'e')).toEqual([
      'Heat',
      'The Third Man',
      'Amélie',
      'Chungking Express',
    ]);
  });

  // The palette opens on what the application can do, and a blank field
  // offering ten thousand films in no particular order is not an answer to
  // anything somebody asked.
  it('finds nothing at all for a query of nothing', () => {
    expect(titlesFor(library, '')).toEqual([]);
    expect(titlesFor(library, '   ')).toEqual([]);
  });

  it('ignores the space either side of a query', () => {
    expect(titlesFor(library, '  heat ')).toEqual(['Heat']);
  });

  it('finds nothing when nothing is called that', () => {
    const found = filmsMatching(entriesOf(library), 'sátántangó');
    expect(found.films).toEqual([]);
    expect(found.withheld).toBe(0);
  });
});

/*
 * A palette is for finding one film rather than for reading a library. What is
 * not drawn is counted and said, since a list that stopped at forty without
 * mentioning it would read as a library of forty films.
 */
describe('a query that matches too much', () => {
  const many = Array.from({ length: SHOWN + 12 }, (_, at) => film(at + 1, `Film ${String(at)}`));

  it('draws as many as it draws, and counts the rest', () => {
    const found = filmsMatching(entriesOf(many), 'film');

    expect(found.films).toHaveLength(SHOWN);
    expect(found.withheld).toBe(12);
  });

  it('withholds nothing when everything matched fits', () => {
    const found = filmsMatching(entriesOf(many), 'film 1');

    expect(found.films.length).toBeLessThan(SHOWN);
    expect(found.withheld).toBe(0);
  });
});

describe('moving the selection', () => {
  it('walks up and down', () => {
    expect(movedBy(1, 1, 5)).toBe(2);
    expect(movedBy(1, -1, 5)).toBe(0);
  });

  // A list reached by the down arrow is a list whose last row is one key from
  // the first.
  it('wraps at both ends', () => {
    expect(movedBy(4, 1, 5)).toBe(0);
    expect(movedBy(0, -1, 5)).toBe(4);
  });

  it('stays at nothing when there is nothing to select', () => {
    expect(movedBy(0, 1, 0)).toBe(0);
    expect(movedBy(0, -1, 0)).toBe(0);
  });
});

/*
 * The figure the whole approach rests on. A substring test over an array
 * prepared once is faster at this size than any index that would have to be
 * built and then kept in step with every scan, and the way to know that is to
 * measure it rather than to argue it.
 *
 * Ten runs and the middle one, so that a machine which pauses for something
 * else during one of them does not decide the result.
 */
describe('ten thousand films', () => {
  const library = Array.from({ length: 10_000 }, (_, at) =>
    film(at + 1, `Film ${String(at).padStart(5, '0')}`, `Shelf ${String(at % 40)}`),
  );

  it('folds the whole library in the time a scan takes to come back', () => {
    const at = performance.now();
    const entries = entriesOf(library);
    const took = performance.now() - at;

    expect(entries).toHaveLength(10_000);
    expect(took).toBeLessThan(100);
  });

  it('answers a keystroke well inside a frame', () => {
    const entries = entriesOf(library);
    const queries = ['f', 'fi', 'film 0', 'film 09', 'film 099', 'shelf 3', 'nothing'];

    const times = queries.flatMap((query) =>
      Array.from({ length: 10 }, () => {
        const at = performance.now();
        filmsMatching(entries, query);
        return performance.now() - at;
      }),
    );

    const middle = [...times].sort((one, other) => one - other)[Math.floor(times.length / 2)];
    expect(middle).toBeLessThan(16);
  });
});
