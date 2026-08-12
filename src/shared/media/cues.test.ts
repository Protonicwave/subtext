import { describe, expect, it } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import {
  AS_WRITTEN,
  type Comfort,
  NONE,
  activeAt,
  nearestFrom,
  nextFrom,
  previousFrom,
  timelineOf,
} from './cues';

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

describe('giving a line long enough to be read', () => {
  /** Enough of each to be obvious in the numbers below. */
  const comfort: Comfort = { leadInMs: 100, minimumMs: 1_000 };

  /** What the lookup would be given, as pairs, so the numbers read plainly. */
  function drawn(cues: readonly CueView[], using: Comfort = comfort): [number, number][] {
    return timelineOf(cues, using).cues.map((shown) => [shown.startMs, shown.endMs]);
  }

  it('puts a line up before it is spoken and holds the end where it was', () => {
    expect(drawn([cue(5_000, 8_000)])).toStrictEqual([[4_900, 8_000]]);
  });

  it('holds a line too short to read for the minimum', () => {
    expect(drawn([cue(5_000, 5_300)])).toStrictEqual([[4_900, 5_900]]);
  });

  it('leaves a line already long enough alone', () => {
    expect(drawn([cue(5_000, 20_000)])).toStrictEqual([[4_900, 20_000]]);
  });

  it('honours the file exactly when asked for nothing', () => {
    const cues = [cue(5_000, 5_100), cue(9_000, 9_200)];

    // The same array, not a copy of it: a film nobody has changed the settings
    // for pays nothing at all for this.
    expect(timelineOf(cues, AS_WRITTEN).cues).toBe(cues);
    expect(timelineOf(cues).cues).toBe(cues);
  });

  it('never puts a line up before the film starts', () => {
    expect(drawn([cue(0, 2_000)])).toStrictEqual([[0, 2_000]]);
    expect(drawn([cue(40, 2_000)])).toStrictEqual([[0, 2_000]]);
  });

  it('takes what lead-in there is where the gap is smaller than it wants', () => {
    // Sixty milliseconds between the two, and a hundred asked for.
    expect(drawn([cue(1_000, 2_000), cue(2_060, 4_000)])).toStrictEqual([
      [900, 2_000],
      [2_000, 4_000],
    ]);
  });

  it('leaves back to back lines back to back', () => {
    // No gap at all, so neither the lead-in nor the minimum has anywhere to go.
    expect(drawn([cue(1_000, 2_000), cue(2_000, 2_400), cue(2_400, 4_000)])).toStrictEqual([
      [900, 2_000],
      [2_000, 2_400],
      [2_400, 4_000],
    ]);
  });

  it('extends a short line as far as the next one and no further', () => {
    // Half a second of room where a full second was wanted.
    expect(drawn([cue(1_000, 1_200), cue(1_700, 3_000)])).toStrictEqual([
      [900, 1_600],
      [1_600, 3_000],
    ]);
  });

  it('never puts two lines on screen that were not there already', () => {
    const cues = [cue(1_000, 1_100), cue(1_500, 1_600), cue(1_650, 1_700), cue(3_000, 3_100)];
    const shown = timelineOf(cues, comfort).cues;

    for (let at = 1; at < shown.length; at += 1) {
      expect(shown[at - 1]?.endMs).toBeLessThanOrEqual(shown[at]?.startMs ?? 0);
    }
  });

  it('leaves the overlaps a file wrote for itself', () => {
    // A sign held under two lines of dialogue. The long cue keeps its end, and
    // the first line, which begins while the sign is up, is not led in over it.
    //
    // The second line is, because the line before it finished long ago. It was
    // already sharing the screen with the sign, so the lead-in is not what put
    // it there and denying it would cost every line under a sign its lead-in.
    expect(drawn([cue(0, 30_000, 'sign'), cue(5_000, 5_200), cue(20_000, 21_000)])).toStrictEqual([
      [0, 30_000],
      [5_000, 6_000],
      [19_900, 21_000],
    ]);
  });

  it('measures the longest cue after the adjustment and not before it', () => {
    // Two hundred milliseconds as written, a second once it is held.
    expect(timelineOf([cue(1_000, 1_200)], comfort).longest).toBe(1_000);
  });

  it('keeps a held line on screen for as long as it is held', () => {
    const held = timelineOf([cue(1_000, 1_200), cue(9_000, 10_000)], comfort);

    expect(activeAt(held, 950)).toBe(0);
    expect(activeAt(held, 1_500)).toBe(0);
    expect(activeAt(held, 1_899)).toBe(0);
    expect(activeAt(held, 1_900)).toBe(NONE);
  });

  it('builds the timeline of a full length film in well under a frame', () => {
    // Two hours of dialogue, adjusted once when the film opens.
    const cues = Array.from({ length: 5_000 }, (_, at) => cue(at * 1_400, at * 1_400 + 1_200));

    // The quickest of several rather than one run of it. The first call carries
    // the engine compiling the loop as well as running it, which on a machine
    // shared with whatever else is building at the time is several times the
    // cost of the work and is not what the two millisecond target is about.
    let quickest = Number.POSITIVE_INFINITY;
    let built = timelineOf(cues, comfort);
    for (let run = 0; run < 5; run += 1) {
      const at = performance.now();
      built = timelineOf(cues, comfort);
      quickest = Math.min(quickest, performance.now() - at);
    }

    expect(built.cues).toHaveLength(5_000);
    expect(quickest).toBeLessThan(2);
  });
});

describe('moving a whole track while somebody is still adjusting it', () => {
  const comfort: Comfort = { leadInMs: 100, minimumMs: 1_000 };

  function drawn(cues: readonly CueView[], shiftMs: number, using = AS_WRITTEN) {
    return timelineOf(cues, using, shiftMs).cues.map((shown) => [shown.startMs, shown.endMs]);
  }

  it('moves every line by the same amount, start and end together', () => {
    expect(drawn([cue(1_000, 2_000), cue(9_000, 10_000)], 500)).toStrictEqual([
      [1_500, 2_500],
      [9_500, 10_500],
    ]);
  });

  it('moves a track that is running ahead of the film back', () => {
    expect(drawn([cue(5_000, 6_000)], -2_500)).toStrictEqual([[2_500, 3_500]]);
  });

  it('keeps a line moved in front of the film at the start of it', () => {
    expect(drawn([cue(1_000, 2_000)], -4_000)).toStrictEqual([[0, 0]]);
  });

  it('changes nothing at all when there is nothing to change', () => {
    const cues = [cue(1_000, 2_000), cue(9_000, 10_000)];

    // The same array, so a film nobody is adjusting pays nothing for the fact
    // that adjusting one is possible.
    expect(timelineOf(cues, AS_WRITTEN, 0).cues).toBe(cues);
  });

  it('gives the moved line the same reading room it would have had', () => {
    // Led in by a hundred and held for a second, both around where the shift
    // put the line rather than where the file did.
    expect(drawn([cue(5_000, 5_300)], 2_000, comfort)).toStrictEqual([[6_900, 7_900]]);
  });

  it('never puts two lines on screen that were not there already', () => {
    const cues = [cue(1_000, 1_100), cue(1_500, 1_600), cue(3_000, 3_100)];
    const shown = timelineOf(cues, comfort, -700).cues;

    for (let at = 1; at < shown.length; at += 1) {
      expect(shown[at - 1]?.endMs).toBeLessThanOrEqual(shown[at]?.startMs ?? 0);
    }
  });

  it('measures the longest cue after the move as well', () => {
    // Both lines are pushed against the start of the film, where the first is
    // squeezed to nothing and the second keeps only what is left of it.
    expect(timelineOf([cue(0, 400), cue(500, 1_500)], AS_WRITTEN, -600).longest).toBe(900);
  });
});
