import { describe, expect, it } from 'vitest';
import { centred } from './crop';

describe('cropping a frame to a poster', () => {
  it('takes a tall rectangle from the middle of a wide frame', () => {
    // Sixteen by nine down to two by three: the height is all of it, and the
    // width is what a two by three rectangle of that height needs.
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
