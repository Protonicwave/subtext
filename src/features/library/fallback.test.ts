import { describe, expect, it } from 'vitest';
import { fallbackFor, initialsOf } from './fallback';

describe('the tile a film gets when no frame could be taken', () => {
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

  it('reduces a title to the letters worth showing', () => {
    expect(initialsOf('Heat')).toBe('H');
    expect(initialsOf('Blade Runner')).toBe('BR');
    // A leading article says nothing about which film it is.
    expect(initialsOf('The Third Man')).toBe('TM');
    expect(initialsOf('La Haine')).toBe('H');
  });

  it('has something to show for a title that is barely one', () => {
    expect(initialsOf('')).toBe('');
    expect(initialsOf('  ')).toBe('');
    expect(initialsOf('12 Angry Men')).toBe('1A');
    expect(initialsOf('the')).toBe('T');
    expect(initialsOf('!!!')).toBe('');
  });
});
