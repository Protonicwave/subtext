import { describe, expect, it } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { NONE, activeAt, timelineOf } from './cues';

function cue(startMs: number, endMs: number, text = 'line'): CueView {
  return { index: 1, startMs, endMs, text, position: null };
}

/** A short film: three lines with a silence in the middle. */
const spoken = timelineOf([
  cue(1_000, 2_000, 'one'),
  cue(2_000, 3_000, 'two'),
  cue(9_000, 10_000, 'three'),
]);

describe('finding the line on screen', () => {
  it('has nothing to say before the first line', () => {
    expect(activeAt(spoken, 0)).toBe(NONE);
    expect(activeAt(spoken, 999)).toBe(NONE);
  });

  it('covers the start of a line and not its end', () => {
    expect(activeAt(spoken, 1_000)).toBe(0);
    expect(activeAt(spoken, 1_999)).toBe(0);
    // The end of one line is the start of the next, and the next wins.
    expect(activeAt(spoken, 2_000)).toBe(1);
    expect(activeAt(spoken, 2_999)).toBe(1);
  });

  it('says nothing during a silence', () => {
    expect(activeAt(spoken, 3_000)).toBe(NONE);
    expect(activeAt(spoken, 6_000)).toBe(NONE);
  });

  it('says nothing after the last line', () => {
    expect(activeAt(spoken, 10_000)).toBe(NONE);
    expect(activeAt(spoken, 9_999_999)).toBe(NONE);
  });

  it('has nothing to say about a film with no subtitles', () => {
    expect(activeAt(timelineOf([]), 1_000)).toBe(NONE);
  });
});

describe('lines that overlap', () => {
  /*
   * A long line held under two short ones, which is how a sign or a song is
   * usually written. Sorted by when they start, the long one comes first and is
   * still on screen when the others begin.
   */
  const overlapping = timelineOf([
    cue(0, 30_000, 'sign'),
    cue(5_000, 6_000, 'first'),
    cue(20_000, 21_000, 'second'),
  ]);

  it('shows the line that has just been spoken', () => {
    expect(activeAt(overlapping, 5_500)).toBe(1);
    expect(activeAt(overlapping, 20_500)).toBe(2);
  });

  it('falls back to the line still running underneath', () => {
    expect(activeAt(overlapping, 2_000)).toBe(0);
    expect(activeAt(overlapping, 10_000)).toBe(0);
    expect(activeAt(overlapping, 25_000)).toBe(0);
  });

  it('stops looking once nothing can still be running', () => {
    expect(activeAt(overlapping, 30_000)).toBe(NONE);
  });
});

describe('the awkward cues a real file holds', () => {
  it('never draws a cue of no length', () => {
    const empty = timelineOf([cue(1_000, 1_000), cue(2_000, 3_000)]);

    expect(activeAt(empty, 1_000)).toBe(NONE);
    expect(activeAt(empty, 2_500)).toBe(1);
  });

  it('measures the longest cue in the track', () => {
    expect(timelineOf([cue(0, 400), cue(1_000, 9_000)]).longest).toBe(8_000);
    expect(timelineOf([]).longest).toBe(0);
  });

  it('finds a line at any point in a full length film', () => {
    // Two hours of dialogue, which is the size the search has to hold up at.
    const many = timelineOf(
      Array.from({ length: 5_000 }, (_, at) =>
        cue(at * 1_400, at * 1_400 + 1_200, `line ${String(at)}`),
      ),
    );

    expect(activeAt(many, 0)).toBe(0);
    expect(activeAt(many, 3_500_000)).toBe(2_500);
    expect(activeAt(many, 3_500_000 + 1_300)).toBe(NONE);
    expect(activeAt(many, 4_999 * 1_400 + 100)).toBe(4_999);
  });
});
