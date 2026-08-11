import { describe, expect, it } from 'vitest';
import type { SearchView } from '@/shared/ipc/bindings';
import { movedBy, rowsOf } from './rows';

const results = {
  shown: 3,
  truncated: false,
  ranked: true,
  films: [
    {
      filmId: 7,
      title: 'Heat',
      year: 1995,
      withheld: 12,
      hits: [
        { startMs: 92_000, snippet: [{ text: 'the action is the juice', matched: false }] },
        { startMs: 415_000, snippet: [{ text: 'a guy told me one time', matched: false }] },
      ],
    },
    {
      filmId: 9,
      title: 'Ronin',
      year: null,
      withheld: 0,
      hits: [{ startMs: 12_000, snippet: [{ text: 'the action is elsewhere', matched: false }] }],
    },
  ],
} satisfies SearchView;

describe('laying results out as one list', () => {
  it('runs the films together in the order they ranked', () => {
    const rows = rowsOf(results);

    expect(rows.map((row) => row.startMs)).toEqual([92_000, 415_000, 12_000]);
    expect(rows.map((row) => row.title)).toEqual(['Heat', 'Heat', 'Ronin']);
  });

  it('marks the line each film starts at, which is what draws the heading', () => {
    expect(rowsOf(results).map((row) => row.opensFilm)).toEqual([true, false, true]);
  });

  it('counts what a film held back on its last line rather than on every one', () => {
    expect(rowsOf(results).map((row) => row.withheld)).toEqual([0, 12, 0]);
  });

  it('has nothing to lay out before a search has been made', () => {
    expect(rowsOf(null)).toEqual([]);
  });
});

describe('moving through the list', () => {
  it('wraps at both ends, because the last line is one key from the first', () => {
    expect(movedBy(0, 1, 3)).toBe(1);
    expect(movedBy(2, 1, 3)).toBe(0);
    expect(movedBy(0, -1, 3)).toBe(2);
  });

  it('stays at nothing when there is nothing to move through', () => {
    expect(movedBy(0, 1, 0)).toBe(0);
    expect(movedBy(0, -1, 0)).toBe(0);
  });
});
