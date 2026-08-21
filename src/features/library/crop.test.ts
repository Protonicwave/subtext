import { describe, expect, it } from 'vitest';
import { centred, fitted } from './crop';

describe('taking a rectangle out of the middle of a frame', () => {
  it('takes a wide rectangle from the middle of a taller frame', () => {
    // Four by three down to sixteen by nine: the width is all of it, and the
    // height is what a sixteen by nine rectangle of that width needs.
    expect(centred(1440, 1080, 16 / 9)).toEqual({ x: 0, y: 135, width: 1440, height: 810 });
  });

  it('takes a tall rectangle from the middle of a wide frame', () => {
    // Sixteen by nine down to two by three: the height is all of it, and the
    // width is what a two by three rectangle of that height needs. No poster is
    // made this way any more, and the arithmetic is the same wherever it is.
    expect(centred(1920, 1080, 2 / 3)).toEqual({ x: 600, y: 0, width: 720, height: 1080 });
  });

  it('takes the whole width when the frame is already narrower than that', () => {
    expect(centred(600, 1200, 2 / 3)).toEqual({ x: 0, y: 150, width: 600, height: 900 });
  });

  it('leaves a square frame alone but for the shape it is asked for', () => {
    expect(centred(1000, 1000, 1)).toEqual({ x: 0, y: 0, width: 1000, height: 1000 });
  });

  it('has an answer for a frame with no size, which is what a failed decode gives', () => {
    expect(centred(0, 0, 2 / 3)).toEqual({ x: 0, y: 0, width: 0, height: 0 });
    expect(centred(1920, 1080, 0)).toEqual({ x: 0, y: 0, width: 0, height: 0 });
  });
});

describe('the size a poster is stored at', () => {
  it('keeps a frame at sixteen by nine rather than cutting it to a cover', () => {
    expect(fitted({ width: 1920, height: 1080 }, 420, 1_260)).toEqual({ width: 420, height: 236 });
  });

  it('keeps artwork at the shape of the cover it already is', () => {
    expect(fitted({ width: 1000, height: 1500 }, 420, 1_260)).toEqual({ width: 420, height: 630 });
  });

  it('brings a picture too tall to be a cover back inside the bound', () => {
    // Anything at all can be dropped on a film. What comes back is the same
    // shape, smaller, rather than the middle of it.
    expect(fitted({ width: 200, height: 2_000 }, 420, 1_260)).toEqual({
      width: 126,
      height: 1_260,
    });
  });

  it('has an answer for a picture with no size, which is what a failed decode gives', () => {
    expect(fitted({ width: 0, height: 0 }, 420, 1_260)).toEqual({ width: 0, height: 0 });
  });
});
