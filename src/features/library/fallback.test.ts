import { describe, expect, it } from 'vitest';
import { fallbackFor } from './fallback';

describe('the colours a film is drawn in before it has a picture', () => {
  it('gives the same film the same colours every time', () => {
    expect(fallbackFor('Heat')).toEqual(fallbackFor('Heat'));
    expect(fallbackFor('Heat').primary).not.toBe(fallbackFor('Ronin').primary);
  });

  it('writes colours a stylesheet can use', () => {
    const { primary, pair } = fallbackFor('The Third Man');

    expect(primary).toMatch(/^#[0-9a-f]{6}$/);
    expect(pair).toMatch(/^#[0-9a-f]{6}$/);
    expect(primary).not.toBe(pair);
  });

  it('has a pair for a title that is barely one', () => {
    for (const title of ['', '  ', '!!!']) {
      expect(fallbackFor(title).primary).toMatch(/^#[0-9a-f]{6}$/);
    }
  });
});
