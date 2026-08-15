import { describe, expect, it } from 'vitest';
import { runtimeOf } from './runtime';

describe('how long a film runs', () => {
  it('says hours and minutes once there is an hour of it', () => {
    expect(runtimeOf(170 * 60_000)).toBe('2 hr 50 min');
    expect(runtimeOf(120 * 60_000)).toBe('2 hr');
    expect(runtimeOf(97 * 60_000)).toBe('1 hr 37 min');
    expect(runtimeOf(44 * 60_000)).toBe('44 min');
  });

  it('says nothing about a film nobody has opened', () => {
    expect(runtimeOf(null)).toBeNull();
    // A running time of nothing at all, which a file that would not decode
    // reports rather than refusing outright.
    expect(runtimeOf(0)).toBeNull();
  });
});
