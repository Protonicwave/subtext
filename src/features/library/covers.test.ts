import { describe, expect, it } from 'vitest';
import type { CoverSourceView, FilmView } from '@/shared/ipc/bindings';
import { coverNameOf, coverStatementOf, coverTallyOf, isChosen } from './covers';

/** Every value the back end can send, so a new one cannot arrive unnamed. */
const SOURCES: CoverSourceView[] = [
  'chosen',
  'in-file',
  'beside',
  'sidecar',
  'folder-above',
  'none',
];

describe('what a cover is said to be', () => {
  it('names every source a film can have', () => {
    for (const source of SOURCES) {
      expect(coverNameOf(source)).not.toBe('');
      expect(coverStatementOf(source)).not.toBe('');
    }
  });

  it('says something different about each of them', () => {
    const names = new Set(SOURCES.map(coverNameOf));
    const statements = new Set(SOURCES.map(coverStatementOf));

    expect(names.size).toBe(SOURCES.length);
    expect(statements.size).toBe(SOURCES.length);
  });

  it('says a film with nothing found is drawn from itself', () => {
    // The one source that is not a picture on the disk, and the one place the
    // wording must not imply that a file was found.
    expect(coverStatementOf('none')).toContain('from the film itself');
  });

  it('counts only a picked cover as a choice to undo', () => {
    expect(isChosen('chosen')).toBe(true);
    for (const source of SOURCES.filter((source) => source !== 'chosen')) {
      expect(isChosen(source)).toBe(false);
    }
  });
});

describe('where a library took its covers from', () => {
  const one = {
    id: 1,
    folderId: 1,
    path: '/films/Heat.1995.mkv',
    shelf: { name: 'Films', path: '/films' },
    title: 'Heat',
    year: 1995,
    addedAt: Date.UTC(2026, 7, 12),
    durationMs: null,
    posterPath: null,
    coverPath: null,
    coverSource: 'none',
    accent: null,
    missing: false,
    details: null,
    tracks: [],
    chosenTrackId: null,
    subtitlesOff: false,
    position: null,
  } satisfies FilmView;

  /** A film covered from one place, with the picture that covered it. */
  function film(id: number, coverSource: CoverSourceView, coverPath: string | null): FilmView {
    return { ...one, id, coverSource, coverPath, title: `Film ${String(id)}` };
  }

  it('counts every film into exactly one place or into the leftovers', () => {
    const films = [
      film(1, 'beside', '/films/Heat.1995.jpg'),
      film(2, 'beside', '/films/Ran.1985.jpg'),
      film(3, 'in-file', '/films/Ronin.1998.mkv'),
      film(4, 'none', null),
    ];

    const { places, leftOver } = coverTallyOf(films);

    const counted = places.reduce((total, place) => total + place.films, 0);
    expect(counted + leftOver.length).toBe(films.length);
    // Best claim first, which is the order the back end decides them in.
    expect(places.map((place) => place.source)).toEqual(['in-file', 'beside']);
    expect(places[1]?.films).toBe(2);
  });

  it('shows one picture for each place, and none where there is no path', () => {
    const { places } = coverTallyOf([
      film(1, 'beside', '/films/Heat.1995.jpg'),
      film(2, 'beside', '/films/Ran.1985.jpg'),
      // A row that names a place and no file, which nothing writes today and
      // which must not be offered as evidence of anything.
      film(3, 'sidecar', null),
    ]);

    expect(places[0]?.example).toBe('/films/Heat.1995.jpg');
    expect(places[1]?.example).toBeNull();
  });

  it('has nothing to say about an empty library', () => {
    expect(coverTallyOf([])).toEqual({ places: [], leftOver: [] });
  });
});
