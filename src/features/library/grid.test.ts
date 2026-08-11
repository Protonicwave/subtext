import { describe, expect, it } from 'vitest';
import { columnsFor, rowHeight, rowsOf } from './grid';

describe('the grid', () => {
  it('fits as many tiles across as the width allows', () => {
    // One tile of 100 and no gap to its right, then 120 per tile after it.
    expect(columnsFor(100, 100, 20)).toBe(1);
    expect(columnsFor(219, 100, 20)).toBe(1);
    expect(columnsFor(220, 100, 20)).toBe(2);
    expect(columnsFor(1000, 100, 20)).toBe(8);
  });

  it('never asks for fewer than one column, whatever the width is', () => {
    expect(columnsFor(0)).toBe(1);
    expect(columnsFor(-500)).toBe(1);
    // Measuring an element that has not been laid out yet gives this.
    expect(columnsFor(Number.NaN)).toBe(1);
  });

  it('makes a row as tall as a tile, what sits under it, and the gap below', () => {
    // Three columns of 100 across 340, each 150 tall at two by three. The gap
    // is part of the row because the virtualiser stacks the rows edge to edge.
    expect(rowHeight(340, 3, 20, 0)).toBe(170);
    expect(rowHeight(340, 3, 20, 30)).toBe(200);
  });

  it('deals the films into rows in the order they were given', () => {
    const films = [1, 2, 3, 4, 5];

    expect(rowsOf(films, 2)).toEqual([[1, 2], [3, 4], [5]]);
    expect(rowsOf(films, 5)).toEqual([films]);
    expect(rowsOf([], 3)).toEqual([]);
  });

  it('deals into one column rather than none when asked for nothing', () => {
    expect(rowsOf([1, 2], 0)).toEqual([[1], [2]]);
  });
});
