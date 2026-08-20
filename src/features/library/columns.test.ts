import { describe, expect, it } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { COLUMNS, ORDER, sortableTitle, sortedBy } from './columns';

const film = {
  id: 1,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  shelf: { name: 'Crime', path: '/films/Crime' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: 170 * 60_000,
  posterPath: null,
  coverSource: 'none',
  accent: null,
  missing: false,
  details: null,
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

function make(id: number, changes: Partial<FilmView>): FilmView {
  return { ...film, id, ...changes };
}

/** What a film's file is, for the columns that read it. */
function details(sizeBytes: number, height: number | null = 1080): FilmView['details'] {
  return {
    container: 'Matroska',
    sizeBytes,
    averageBitrate: null,
    video: { codec: 'H.264', width: null, height, bitDepth: 8, frameRate: null },
    audio: [],
  };
}

/** The titles, in the order the column put them. */
function titles(films: FilmView[]): string[] {
  return films.map((sorted) => sorted.title);
}

describe('the columns of the list', () => {
  it('has one for every sort that can be remembered', () => {
    expect(ORDER).toHaveLength(Object.keys(COLUMNS).length);
    expect(new Set(ORDER)).toEqual(new Set(Object.keys(COLUMNS)));
  });

  it('orders by title, and reverses', () => {
    const films = [make(1, { title: 'Stalker' }), make(2, { title: 'Heat' })];

    expect(titles(sortedBy(films, 'title', false))).toEqual(['Heat', 'Stalker']);
    expect(titles(sortedBy(films, 'title', true))).toEqual(['Stalker', 'Heat']);
  });

  /*
   * The Conversation belongs under C, the way it does on a shelf. This is the
   * one comparison in the table that is not simply what the cell says.
   */
  it('files a title under its first real word rather than under its article', () => {
    const films = [
      make(1, { title: 'The Conversation' }),
      make(2, { title: 'Blow Out' }),
      make(3, { title: 'A Prophet' }),
      make(4, { title: 'An Elephant Sitting Still' }),
    ];

    expect(titles(sortedBy(films, 'title', false))).toEqual([
      'Blow Out',
      'The Conversation',
      'An Elephant Sitting Still',
      'A Prophet',
    ]);
  });

  it('keeps two titles that differ only by an article in a settled order', () => {
    const films = [make(1, { title: 'The Thing' }), make(2, { title: 'Thing' })];
    const forwards = titles(sortedBy(films, 'title', false));

    expect(forwards).toEqual(titles(sortedBy([...films].reverse(), 'title', false)));
    expect(forwards).toEqual(['The Thing', 'Thing']);
  });

  it('reads a title that is nothing but an article as it stands', () => {
    expect(sortableTitle('The')).toBe('The');
    expect(sortableTitle('A')).toBe('A');
    expect(sortableTitle('The Conversation')).toBe('Conversation');
  });

  it('ignores case and accents, so a library is in one order rather than two', () => {
    const films = [
      make(1, { title: 'Ördet' }),
      make(2, { title: 'la haine' }),
      make(3, { title: 'Persona' }),
    ];

    expect(titles(sortedBy(films, 'title', false))).toEqual(['la haine', 'Ördet', 'Persona']);
  });

  it('orders figures as numbers rather than as the text of them', () => {
    const films = [
      make(1, { title: 'Nine', durationMs: 9 * 60_000 }),
      make(2, { title: 'Ten', durationMs: 10 * 60_000 }),
      make(3, { title: 'Two', durationMs: 2 * 60_000 }),
    ];

    expect(titles(sortedBy(films, 'runtime', false))).toEqual(['Two', 'Nine', 'Ten']);
  });

  /*
   * The awkward half of the table. A film recorded before this build read what
   * its file was has no size, and a file with no year in its name has no year.
   */
  it('puts a film with no year last, whichever way round the column is', () => {
    const films = [
      make(1, { title: 'Unknown', year: null }),
      make(2, { title: 'Older', year: 1974 }),
      make(3, { title: 'Newer', year: 2015 }),
    ];

    expect(titles(sortedBy(films, 'year', false))).toEqual(['Older', 'Newer', 'Unknown']);
    expect(titles(sortedBy(films, 'year', true))).toEqual(['Newer', 'Older', 'Unknown']);
  });

  it('puts a film whose file has not been read last in the columns that read it', () => {
    const films = [
      make(1, { title: 'Unread', details: null }),
      make(2, { title: 'Small', details: details(1_000_000) }),
      make(3, { title: 'Large', details: details(9_000_000_000) }),
    ];

    expect(titles(sortedBy(films, 'size', false))).toEqual(['Small', 'Large', 'Unread']);
    expect(titles(sortedBy(films, 'size', true))).toEqual(['Large', 'Small', 'Unread']);
  });

  it('settles films with nothing to compare by their titles', () => {
    const films = [
      make(1, { title: 'Stalker', year: null }),
      make(2, { title: 'Heat', year: null }),
    ];

    expect(titles(sortedBy(films, 'year', false))).toEqual(['Heat', 'Stalker']);
    expect(titles(sortedBy(films, 'year', true))).toEqual(['Heat', 'Stalker']);
  });

  it('settles films that share a value by their titles rather than by chance', () => {
    const films = [
      make(1, { title: 'Stalker', year: 1979 }),
      make(2, { title: 'Alien', year: 1979 }),
      make(3, { title: 'Manhattan', year: 1979 }),
    ];

    expect(titles(sortedBy(films, 'year', false))).toEqual(['Alien', 'Manhattan', 'Stalker']);
  });

  it('orders the picture by how tall it is, since that is the fact in the column', () => {
    const films = [
      make(1, { title: 'High', details: details(1, 2160) }),
      make(2, { title: 'Standard', details: details(1, 576) }),
      make(3, { title: 'Sizeless', details: details(1, null) }),
    ];

    expect(titles(sortedBy(films, 'video', true))).toEqual(['High', 'Standard', 'Sizeless']);
  });

  /*
   * A film that was finished is all of it whatever second it stopped at, and a
   * film nobody has opened has no place among the ones somebody is part way
   * through.
   */
  it('orders by how much of a film has been watched', () => {
    const at = (progress: number | null, finished = false) => ({
      positionMs: 1,
      durationMs: 100,
      finished,
      updatedAt: 0,
      progress,
    });

    const films = [
      make(1, { title: 'Started', position: at(0.1) }),
      make(2, { title: 'Finished', position: at(0.4, true) }),
      make(3, { title: 'Untouched', position: null }),
      make(4, { title: 'Halfway', position: at(0.5) }),
    ];

    expect(titles(sortedBy(films, 'watched', false))).toEqual([
      'Started',
      'Halfway',
      'Finished',
      'Untouched',
    ]);
  });

  it('leaves the library it was given alone', () => {
    const films = [make(1, { title: 'Stalker' }), make(2, { title: 'Heat' })];
    sortedBy(films, 'title', false);

    expect(titles(films)).toEqual(['Stalker', 'Heat']);
  });

  /*
   * Ten thousand films are a hundred and thirty thousand comparisons, and a
   * sort somebody can see happen is a sort that has gone wrong.
   *
   * What this guards is the shape of the comparison rather than the speed of
   * the machine. A comparator that derives a film's sort key on every step
   * instead of reading a value costs a multiple of the whole sort, so a
   * regression of that kind shows up as seconds and not as milliseconds.
   *
   * The bound is set from that and not from the frame budget. Sorting this
   * library takes about eight milliseconds on a developer's machine, and a
   * shared build runner has been seen to take fifty on a single sample, which
   * is the runner and not the code: nothing about a comparison changes because
   * another job is running beside it. A bound near the frame budget therefore
   * measures the hardware, and it failed on a green tree twice. This one sits
   * an order of magnitude above the true cost, where only a real regression
   * reaches it.
   *
   * The figure the screen actually rests on is in the measured pass over a real
   * library, which is where a claim about a frame belongs. It is not something
   * a unit test on a borrowed machine can say.
   */
  it('sorts ten thousand films without deriving each one again', () => {
    const many = Array.from({ length: 10_000 }, (_, at) =>
      make(at + 1, {
        title: `Film ${String((at * 7919) % 10_000)}`,
        year: 1950 + (at % 70),
        details: details((at % 500) * 20_000_000),
      }),
    );

    for (const name of ORDER) sortedBy(many, name, false);

    const at = performance.now();
    const sorted = sortedBy(many, 'title', false);
    const took = performance.now() - at;

    expect(sorted).toHaveLength(10_000);
    expect(took).toBeLessThan(400);
  });
});
