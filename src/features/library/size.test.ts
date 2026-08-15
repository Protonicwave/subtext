import { describe, expect, it } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { libraryTotalOf, sizeOf } from './size';

/** A film of a given size, or one this build has not looked at. */
function weighs(sizeBytes: number | null): FilmView {
  return {
    details:
      sizeBytes === null
        ? null
        : { container: 'mkv', sizeBytes, averageBitrate: null, video: null, audio: [] },
  } as FilmView;
}

describe('how large something is', () => {
  it('says it the way a drive says it', () => {
    expect(sizeOf(15_800_000_000)).toBe('15.8 GB');
    expect(sizeOf(1_000_000_000)).toBe('1.0 GB');
    expect(sizeOf(742_000_000)).toBe('742 MB');
    expect(sizeOf(48_000)).toBe('48 kB');
    expect(sizeOf(400)).toBe('400 bytes');
  });

  it('says nothing strange about nothing at all', () => {
    expect(sizeOf(0)).toBe('0 bytes');
  });
});

describe('how large a library is', () => {
  it('adds up what every film weighs', () => {
    expect(libraryTotalOf([weighs(2_000_000_000), weighs(3_000_000_000)])).toBe(5_000_000_000);
  });

  /*
   * A film recorded before this build described such things has no size until
   * its folder is read again, and a total that quietly left those out would be
   * wrong without saying so.
   */
  it('gives no total at all where the size of one film is not known', () => {
    expect(libraryTotalOf([weighs(2_000_000_000), weighs(null)])).toBeNull();
  });

  it('has no total for a library with nothing in it', () => {
    expect(libraryTotalOf([])).toBeNull();
  });
});
