import { describe, expect, it } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { BUCKETS, shapeOf, talkingOf } from './dialogue';

function cue(startMs: number, endMs: number): CueView {
  return { index: 1, startMs, endMs, text: 'I take scores.', position: null };
}

describe('where a film does its talking', () => {
  it('puts the dialogue in the part of the film it was spoken in', () => {
    // Ten minutes of film, and every line of it in the first minute.
    const talking = talkingOf([cue(0, 30_000), cue(30_000, 60_000)], 600_000, 10);

    expect(talking[0]).toBe(1);
    expect(talking.slice(1)).toEqual([0, 0, 0, 0, 0, 0, 0, 0, 0]);
  });

  it('measures every bucket against the busiest one', () => {
    // The second minute is spoken over twice as much as the first, and the
    // first half as much as the second rather than a fifth of a bucket.
    const talking = talkingOf([cue(0, 30_000), cue(60_000, 120_000)], 600_000, 10);

    expect(talking[0]).toBe(0.5);
    expect(talking[1]).toBe(1);
  });

  it('spreads a line that runs across a boundary over both sides of it', () => {
    const talking = talkingOf([cue(30_000, 90_000)], 600_000, 10);

    expect(talking[0]).toBe(1);
    expect(talking[1]).toBe(1);
  });

  it('draws nothing for a cue that covers no time', () => {
    // A file that stops mid-cue leaves lines like this behind. The player does
    // not draw one over the picture either, so the shape does not claim it.
    expect(talkingOf([cue(300_000, 300_000)], 600_000, 10)).toEqual(new Array<number>(10).fill(0));
  });

  it('draws a film with one line as one line', () => {
    const talking = talkingOf([cue(0, 60_000)], 600_000, 10);

    expect(talking[0]).toBe(1);
    expect(talking.filter((filled) => filled > 0)).toHaveLength(1);
  });

  it('has nothing to draw for a track with no lines in it', () => {
    expect(talkingOf([], 600_000, 10)).toEqual(new Array<number>(10).fill(0));
  });

  it('keeps a track that runs past the end of the film inside the film', () => {
    // A subtitle for a longer cut, or one nudged forward. It piles into the
    // last bucket rather than reaching past it or being dropped.
    const talking = talkingOf([cue(590_000, 900_000)], 600_000, 10);

    expect(talking).toHaveLength(10);
    expect(talking[9]).toBe(1);
    expect(talking.every((filled) => filled >= 0 && filled <= 1)).toBe(true);
  });

  it('has nothing to draw until the film says how long it is', () => {
    // The length comes from the element once it has read the file, so the bar
    // is drawn at least once before there is anywhere to put the dialogue.
    expect(talkingOf([cue(0, 60_000)], null, 10)).toEqual([]);
    expect(talkingOf([cue(0, 60_000)], 0, 10)).toEqual([]);
  });
});

describe('the shape drawn from it', () => {
  it('is one path, whatever the length of the film', () => {
    const many = Array.from({ length: 5_000 }, (_, line) =>
      cue(line * 1_400, line * 1_400 + 1_200),
    );

    const started = performance.now();
    const shape = shapeOf(talkingOf(many, 7_000_000));
    const took = performance.now() - started;

    expect(shape.startsWith('M0,0L')).toBe(true);
    expect(shape.endsWith(`L${String(BUCKETS)},0Z`)).toBe(true);
    // One point a bucket, and the two corners it hangs from.
    expect(shape.split('L')).toHaveLength(BUCKETS + 2);

    // Well inside the three milliseconds this is given, and loose enough not to
    // fail on a busy machine.
    expect(took).toBeLessThan(50);
  });

  it('draws the busiest bucket at its full height and a quiet one at nothing', () => {
    expect(shapeOf([1, 0])).toBe('M0,0L0.5,100L1.5,0L2,0Z');
  });

  it('draws nothing for a film with nothing to draw', () => {
    expect(shapeOf([])).toBe('');
  });
});
