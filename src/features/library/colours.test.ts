import { describe, expect, it } from 'vitest';
import { accentFrom, hex, toHsl } from './colours';

/** A block of pixels in the given colours, one after another. */
function frame(...colours: [number, number, number][]): Uint8ClampedArray {
  const pixels = new Uint8ClampedArray(colours.length * 4);
  colours.forEach(([red, green, blue], at) => {
    pixels.set([red, green, blue, 255], at * 4);
  });
  return pixels;
}

/** How many degrees round the wheel a colour sits. */
function hueOf(colour: string): number {
  const value = Number.parseInt(colour.slice(1), 16);
  return toHsl((value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff).h;
}

/** How far apart two hues are, the short way round. */
function apart(one: string, other: string): number {
  const difference = Math.abs(hueOf(one) - hueOf(other));
  return Math.min(difference, 360 - difference);
}

describe('the colour a frame reads as', () => {
  it('takes the hue that carries the frame', () => {
    const accent = accentFrom(frame([200, 30, 30], [190, 40, 35], [205, 25, 25]));

    expect(accent).not.toBeNull();
    expect(apart(accent?.primary ?? '#000000', '#ff0000')).toBeLessThan(20);
  });

  it('has nothing to say about a frame with no colour in it', () => {
    // A black and white film, and a frame that is a fade to black.
    expect(accentFrom(frame([90, 90, 90], [180, 180, 180], [20, 20, 20]))).toBeNull();
    expect(accentFrom(frame([0, 0, 0], [2, 2, 2]))).toBeNull();
    expect(accentFrom(new Uint8ClampedArray(0))).toBeNull();
  });

  it('weighs a vivid colour above the neutral mass around it', () => {
    // What a night interior looks like: mostly dark grey, one lit window.
    const dull: [number, number, number][] = Array.from({ length: 60 }, () => [34, 36, 40]);
    const accent = accentFrom(frame(...dull, [40, 140, 220], [45, 130, 215]));

    expect(apart(accent?.primary ?? '#000000', '#2b8cdc')).toBeLessThan(25);
  });

  it('picks a second colour that is not the first one again', () => {
    const accent = accentFrom(frame([210, 40, 40], [200, 45, 35], [40, 90, 200], [45, 95, 205]));

    expect(apart(accent?.primary ?? '#000000', accent?.pair ?? '#000000')).toBeGreaterThan(50);
  });

  it('invents a second colour when the frame only has one', () => {
    const accent = accentFrom(frame([200, 30, 30], [195, 35, 35]));

    expect(apart(accent?.primary ?? '#000000', accent?.pair ?? '#000000')).toBeGreaterThan(50);
  });

  it('lifts a colour too dark to see into a range that can be drawn', () => {
    // A deep maroon, which as a glow behind a tile would be invisible.
    const accent = accentFrom(frame([40, 6, 6], [44, 8, 8]));
    const lightness = toHsl(
      Number.parseInt((accent?.primary ?? '#000000').slice(1, 3), 16),
      Number.parseInt((accent?.primary ?? '#000000').slice(3, 5), 16),
      Number.parseInt((accent?.primary ?? '#000000').slice(5, 7), 16),
    ).l;

    expect(lightness).toBeGreaterThan(0.4);
  });

  it('writes a colour the same way every time', () => {
    expect(hex(0, 1, 0.5)).toMatch(/^#[0-9a-f]{6}$/);
    // Round the wheel and back again is the same colour.
    expect(hex(400, 0.5, 0.5)).toBe(hex(40, 0.5, 0.5));
    expect(hex(-40, 0.5, 0.5)).toBe(hex(320, 0.5, 0.5));
  });
});
