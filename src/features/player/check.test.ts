import { describe, expect, it } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { LEAD_MS, WATCH_MS, busiestMomentOf } from './check';

/** A line, since nothing here is about what any of them say. */
function line(startMs: number, index: number): CueView {
  return { index, startMs, endMs: startMs + 1_500, text: 'line', position: null };
}

/** A track whose lines fall at the given moments, in playback order. */
function track(starts: readonly number[]): CueView[] {
  return starts.map((startMs, at) => line(startMs, at + 1));
}

describe('the moment a measurement is judged by', () => {
  /*
   * The whole point of choosing a moment rather than staying where the film is.
   * A line every ten seconds takes a minute to form an opinion about; four
   * people talking over each other takes one line.
   */
  it('goes to the busiest stretch of dialogue in the film', () => {
    const quiet = [0, 60_000, 120_000, 180_000];
    const busy = [600_000, 602_000, 604_000, 606_000, 608_000, 610_000];

    expect(busiestMomentOf(track([...quiet, ...busy]))).toBe(600_000 - LEAD_MS);
  });

  /*
   * A line that is already on screen when the film arrives cannot be told from
   * one that came a quarter of a second late, and arriving is the thing being
   * judged. So the film is put a moment before the first line rather than on it.
   */
  it('starts a little before the first line, so it is seen to arrive', () => {
    expect(busiestMomentOf(track([30_000, 31_000, 32_000]))).toBe(30_000 - LEAD_MS);
  });

  /** A film whose dialogue begins in the first second has nowhere to back up to. */
  it('never asks for a moment before the film starts', () => {
    expect(busiestMomentOf(track([200, 900, 1_400]))).toBe(0);
  });

  it('has nowhere to send anybody when the track has no lines', () => {
    expect(busiestMomentOf([])).toBeNull();
  });

  it('sends them to the only line there is where that is all there is', () => {
    expect(busiestMomentOf(track([420_000]))).toBe(420_000 - LEAD_MS);
  });

  /*
   * Two stretches with the same amount of talking in them are equally good
   * evidence, and the earlier one is nearer wherever somebody already is.
   */
  it('takes the earlier of two stretches that are as busy as each other', () => {
    const twice = [...[100_000, 101_000, 102_000], ...[900_000, 901_000, 902_000]];

    expect(busiestMomentOf(track(twice))).toBe(100_000 - LEAD_MS);
  });

  /*
   * The window is what makes density mean anything. Lines spread further apart
   * than it is are not a scene of people talking, however many of them there
   * are in a row.
   */
  it('counts only the lines that fall inside the stretch it would play', () => {
    const spread = [0, WATCH_MS, WATCH_MS * 2, WATCH_MS * 3, WATCH_MS * 4];
    const together = [500_000, 505_000, 510_000];

    expect(busiestMomentOf(track([...spread, ...together]))).toBe(500_000 - LEAD_MS);
  });
});
