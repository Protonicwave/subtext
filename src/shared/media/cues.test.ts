import { describe, expect, it } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { NONE, activeAt, nearestFrom, nextFrom, previousFrom, timelineOf } from './cues';

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

describe('stepping through a film by its dialogue', () => {
  it('goes on to the line after this one', () => {
    expect(nextFrom(spoken, 0)).toBe(0);
    expect(nextFrom(spoken, 1_500)).toBe(1);
    // From a silence, on to whatever is said next.
    expect(nextFrom(spoken, 5_000)).toBe(2);
  });

  it('has nowhere to go on to after the last line', () => {
    expect(nextFrom(spoken, 9_500)).toBe(NONE);
    expect(nextFrom(spoken, 60_000)).toBe(NONE);
  });

  it('goes back to the start of the line being spoken', () => {
    // Part way through the second line, which is the line that was missed.
    expect(previousFrom(spoken, 2_600)).toBe(1);
  });

  it('steps behind the line it has just gone back to', () => {
    // Pressing it again from where the last one landed, which is the whole
    // point of the grace: without it this would stay on the same line for ever.
    expect(previousFrom(spoken, 2_000)).toBe(0);
    expect(previousFrom(spoken, 1_000)).toBe(NONE);
  });

  it('goes back to the last thing said during a silence', () => {
    expect(previousFrom(spoken, 6_000)).toBe(1);
  });

  it('has nothing to step through in a film with no subtitles', () => {
    const silent = timelineOf([]);

    expect(nextFrom(silent, 0)).toBe(NONE);
    expect(previousFrom(silent, 60_000)).toBe(NONE);
  });
});

describe('the line spoken around a moment', () => {
  const within = 4_000;

  it('is the line on screen when there is one', () => {
    expect(nearestFrom(spoken, 1_500, within)).toBe(0);
    expect(nearestFrom(spoken, 9_999, within)).toBe(2);
  });

  it('is whichever line is nearer when the film is quiet', () => {
    // Three seconds past the second line and six short of the third.
    expect(nearestFrom(spoken, 6_000, within)).toBe(1);
    // A second short of the third line and six past the second.
    expect(nearestFrom(spoken, 8_000, within)).toBe(2);
  });

  it('says nothing at all about a long silence', () => {
    const quiet = timelineOf([cue(0, 1_000), cue(600_000, 601_000)]);

    expect(nearestFrom(quiet, 300_000, within)).toBe(NONE);
  });

  it('reaches back past the end of the film', () => {
    expect(nearestFrom(spoken, 12_000, within)).toBe(2);
    expect(nearestFrom(spoken, 60_000, within)).toBe(NONE);
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

  it('answers a frame of a full length film in well under a frame', () => {
    // Two hours of dialogue, looked at once per frame for four minutes of it.
    // The whole point of the binary search is that this is nothing: the
    // transcript, the subtitles and the frame loop all rest on it.
    const many = timelineOf(
      Array.from({ length: 5_000 }, (_, at) => cue(at * 1_400, at * 1_400 + 1_200)),
    );

    const at = performance.now();
    for (let frame = 0; frame < 15_000; frame += 1) activeAt(many, frame * 466);
    const each = (performance.now() - at) / 15_000;

    // A frame at sixty a second is 16.7ms, and this is a small fraction of one
    // per cent of that even on a machine with something else running.
    expect(each).toBeLessThan(0.05);
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
