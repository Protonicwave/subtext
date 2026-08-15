import { describe, expect, it } from 'vitest';
import { dateOf } from './dates';

// Local rather than in UTC, since a date is shown in the time the machine
// keeps and a fixture written the other way would read as the day before on
// half the machines that run this.
const today = new Date(2026, 7, 15);

describe('the date a column shows', () => {
  it('gives the day and the month for something from this year', () => {
    expect(dateOf(new Date(2026, 7, 12).getTime(), today)).toBe('12 Aug');
    expect(dateOf(new Date(2026, 0, 1).getTime(), today)).toBe('1 Jan');
  });

  /*
   * A library filled over years wants to know which of them, and one filled
   * last week does not want a column of the same number repeated down it.
   */
  it('adds the year once the date is not in this one', () => {
    expect(dateOf(new Date(2019, 10, 3).getTime(), today)).toBe('3 Nov 2019');
  });

  it('says nothing about a date it cannot read', () => {
    expect(dateOf(Number.NaN, today)).toBe('');
  });
});
