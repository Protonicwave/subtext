import { describe, expect, it } from 'vitest';
import { clockOf, countdownOf } from './clock';

describe('the elapsed figure', () => {
  it('leaves the hours out until there are some', () => {
    expect(clockOf(0)).toBe('0:00');
    expect(clockOf(9_000)).toBe('0:09');
    expect(clockOf(252_000)).toBe('4:12');
    expect(clockOf(3_599_000)).toBe('59:59');
  });

  it('pads the minutes once an hour is showing', () => {
    expect(clockOf(3_600_000)).toBe('1:00:00');
    expect(clockOf(3_852_000)).toBe('1:04:12');
    expect(clockOf(12_600_000)).toBe('3:30:00');
  });

  it('counts whole seconds, and rounds down like a clock does', () => {
    expect(clockOf(1_999)).toBe('0:01');
  });

  it('draws digits for a file that will not say how long it is', () => {
    expect(clockOf(Number.NaN)).toBe('0:00');
    expect(clockOf(Number.POSITIVE_INFINITY)).toBe('0:00');
    expect(clockOf(-5_000)).toBe('0:00');
  });
});

describe('the remaining figure', () => {
  it('counts down from the running time', () => {
    expect(countdownOf(0, 3_852_000)).toBe('-1:04:12');
    expect(countdownOf(252_000, 3_852_000)).toBe('-1:00:00');
  });

  it('rounds up, so the last second is shown rather than skipped', () => {
    expect(countdownOf(1_500, 3_000)).toBe('-0:02');
    expect(countdownOf(2_999, 3_000)).toBe('-0:01');
    expect(countdownOf(3_000, 3_000)).toBe('-0:00');
  });

  it('never goes past the end, for a position saved from a longer cut', () => {
    expect(countdownOf(9_000, 3_000)).toBe('-0:00');
  });

  it('says nothing until the running time is known', () => {
    expect(countdownOf(1_000, null)).toBe('--:--');
  });
});
