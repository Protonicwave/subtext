import { describe, expect, it } from 'vitest';
import { estimateHeight, inView } from './rows';

describe('guessing how tall a line will be', () => {
  it('gives a short line one row of text', () => {
    expect(estimateHeight('I rob banks.')).toBe(estimateHeight('Yes.'));
  });

  it('gives a long line room to wrap', () => {
    const long = 'A guy told me one time, do not let yourself get attached to anything.';

    expect(estimateHeight(long)).toBeGreaterThan(estimateHeight('I rob banks.'));
  });

  it('keeps a break the file made', () => {
    // Two short lines are two rows, where the same words in one run would fit
    // on one. The panel draws the break, so the guess has to expect it.
    expect(estimateHeight('What do you do?\nI rob banks.')).toBeGreaterThan(
      estimateHeight('What do you do? I rob banks.'),
    );
  });

  it('gives a cue with nothing in it a row all the same', () => {
    expect(estimateHeight('')).toBeGreaterThan(0);
  });
});

describe('whether the film has scrolled out from under the reader', () => {
  it('counts a row inside the view as seen', () => {
    expect(inView(400, 0, 768)).toBe(true);
  });

  it('counts a row below the view as gone', () => {
    expect(inView(900, 0, 768)).toBe(false);
  });

  it('counts a row scrolled off the top as gone', () => {
    expect(inView(100, 400, 768)).toBe(false);
  });

  it('counts the first and last rows of the view as seen', () => {
    expect(inView(400, 400, 768)).toBe(true);
    expect(inView(1_168, 400, 768)).toBe(true);
  });

  it('finds nothing in view in a panel of no height', () => {
    // What a panel that has not been laid out reports. Nothing is in front of
    // anybody, and the answer is the honest one rather than the convenient one.
    expect(inView(40, 0, 0)).toBe(false);
  });
});
