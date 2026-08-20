import { describe, expect, it } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { mastheadOf, CARRYING_ON, shelvesOf } from './shelves';

/**
 * Only the fields the arrangement reads are given, since a whole film view here
 * would say that the identifier, the shelf and whether the file is there are
 * one thing among twenty rather than the three this module is about.
 */
function film(id: number, shelf: string, extra: Partial<FilmView> = {}): FilmView {
  const parts = shelf.split('/');

  return {
    id,
    title: `Film ${String(id)}`,
    missing: false,
    shelf: { name: parts[parts.length - 1] ?? shelf, path: shelf },
    ...extra,
  } as FilmView;
}

describe('arranging the library into shelves', () => {
  it('gathers the films of one folder onto one row', () => {
    const shelves = shelvesOf([film(1, '/films/Crime'), film(2, '/films/Crime')], []);

    expect(shelves).toHaveLength(1);
    expect(shelves[0]?.name).toBe('Crime');
    expect(shelves[0]?.path).toBe('/films/Crime');
    expect(shelves[0]?.films.map((held) => held.id)).toEqual([1, 2]);
  });

  /*
   * The library arrives in alphabetical order, so the first film of a row says
   * nothing about when that row was first met. The lowest identifier does.
   */
  it('orders the rows by when each folder was first met on disk', () => {
    const shelves = shelvesOf(
      [
        film(9, '/films/Crime'),
        film(2, '/films/Epics'),
        film(4, '/films/Crime'),
        film(7, '/films/Epics'),
      ],
      [],
    );

    expect(shelves.map((shelf) => shelf.name)).toEqual(['Epics', 'Crime']);
  });

  it('keeps two folders of the same name apart', () => {
    const shelves = shelvesOf([film(1, '/films/Crime'), film(2, '/other/Crime')], []);

    expect(shelves).toHaveLength(2);
    expect(shelves.map((shelf) => shelf.path)).toEqual(['/films/Crime', '/other/Crime']);
  });

  it('puts what there is to carry on with above the folders, in the order given', () => {
    const [stopped, other] = [film(4, '/films/Crime'), film(1, '/films/Epics')];
    const shelves = shelvesOf([other, stopped], [stopped, other]);

    expect(shelves[0]?.key).toBe(CARRYING_ON);
    expect(shelves[0]?.path).toBeNull();
    expect(shelves[0]?.films.map((held) => held.id)).toEqual([4, 1]);
    expect(shelves.slice(1).map((shelf) => shelf.name)).toEqual(['Epics', 'Crime']);
  });

  it('leaves the row out entirely when nothing has been started', () => {
    const shelves = shelvesOf([film(1, '/films/Crime')], []);

    expect(shelves.every((shelf) => shelf.key !== CARRYING_ON)).toBe(true);
  });

  it('has nothing to arrange when the library is empty', () => {
    expect(shelvesOf([], [])).toEqual([]);
  });
});

describe('choosing the film shown large', () => {
  it('takes what was stopped most recently', () => {
    const stopped = film(2, '/films/Crime');
    expect(mastheadOf([film(5, '/films/Epics'), stopped], [stopped])).toBe(stopped);
  });

  it('takes the most recently added film when nothing has been started', () => {
    const newest = film(5, '/films/Epics');
    expect(mastheadOf([film(2, '/films/Crime'), newest], [])).toBe(newest);
  });

  /*
   * The masthead offers to carry on with what it shows, and a file that is not
   * there is the one thing it should not offer.
   */
  it('never shows a film whose file has gone', () => {
    const here = film(1, '/films/Crime');
    const gone = film(9, '/films/Crime', { missing: true });

    expect(mastheadOf([here, gone], [gone])).toBe(here);
    expect(mastheadOf([gone], [])).toBeNull();
  });

  it('shows nothing at all when there is nothing to show', () => {
    expect(mastheadOf([], [])).toBeNull();
  });
});
