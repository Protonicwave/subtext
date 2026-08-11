import { describe, expect, it } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { BANDS, densityOf, shapeFor, shapeOf } from './density';

function cue(startMs: number, endMs: number): CueView {
  return { index: 1, startMs, endMs, text: 'line', position: null };
}

/** An hour, in bands of thirty seconds. */
const HOUR = 3_600_000;
const BAND = HOUR / BANDS;

describe('where the dialogue falls in a film', () => {
  it('puts a line in the band it is spoken in', () => {
    const map = densityOf([cue(0, BAND)], HOUR);

    expect(map[0]).toBe(1);
    expect(map[1]).toBe(0);
    expect(map).toHaveLength(BANDS);
  });

  it('draws a quiet stretch as a valley between two busy ones', () => {
    const map = densityOf([cue(0, BAND), cue(BAND, BAND * 2), cue(BAND * 8, BAND * 9)], HOUR);

    expect(map[0]).toBe(1);
    expect(map[1]).toBe(1);
    // The silence in between, which is the whole point of drawing this.
    expect(map.slice(2, 8)).toEqual([0, 0, 0, 0, 0, 0]);
    expect(map[8]).toBe(1);
  });

  it('shares a line that runs over a boundary between the bands it covers', () => {
    const map = densityOf([cue(BAND / 2, BAND * 1.5)], HOUR);

    expect(map[0]).toBeCloseTo(1);
    expect(map[1]).toBeCloseTo(1);
  });

  it('measures each film against its own busiest moment', () => {
    // A quiet film, where the most that is ever said in half a minute is one
    // short line. It should still reach the top of the bar somewhere.
    const map = densityOf([cue(0, 800), cue(BAND * 4, BAND * 4 + 400)], HOUR);

    expect(Math.max(...map)).toBe(1);
    expect(map[4]).toBeCloseTo(0.5);
  });

  it('has nothing to draw for a film with no dialogue or no length', () => {
    expect(densityOf([], HOUR).every((held) => held === 0)).toBe(true);
    expect(densityOf([cue(0, 1_000)], null).every((held) => held === 0)).toBe(true);
    expect(densityOf([cue(0, 1_000)], 0).every((held) => held === 0)).toBe(true);
  });

  it('keeps a line that runs past the end of the film inside the bar', () => {
    // A subtitle file for a longer cut than the one on disk, which is common
    // enough to be worth not drawing outside the bar.
    const map = densityOf([cue(HOUR - 100, HOUR * 2)], HOUR);

    expect(map).toHaveLength(BANDS);
    expect(map[BANDS - 1]).toBe(1);
  });

  it('measures a full length film in one pass', () => {
    const many = Array.from({ length: 5_000 }, (_, at) => cue(at * 1_400, at * 1_400 + 1_200));

    const at = performance.now();
    densityOf(many, 7_000_000);
    expect(performance.now() - at).toBeLessThan(50);
  });
});

describe('the shape the scrubber draws', () => {
  it('is a closed path about the middle of the bar', () => {
    const shape = shapeOf([1, 0]);

    expect(shape.startsWith('M')).toBe(true);
    expect(shape.endsWith('Z')).toBe(true);
    // Above and below for each band: the shape is mirrored, not stood on the
    // floor of the bar.
    expect(shape.split('L')).toHaveLength(4);
  });

  it('leaves a silent stretch as a line rather than as a gap', () => {
    // The scrubber is still something to be dragged, whatever the film does.
    expect(shapeOf([0])).toBe('M0.5,47L0.5,53Z');
  });

  it('has nothing to draw for a film that was never measured', () => {
    expect(shapeOf([])).toBe('');
  });

  it('works a film out once and remembers it', () => {
    const cues = [cue(0, 1_000)];
    const first = shapeFor('7:3600000', cues, HOUR);

    // The same film again, with nothing to work it out from. What comes back is
    // what was worked out the first time.
    expect(shapeFor('7:3600000', [], HOUR)).toBe(first);
    expect(shapeFor('8:3600000', [], HOUR)).not.toBe(first);
  });
});
