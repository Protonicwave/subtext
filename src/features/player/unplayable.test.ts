import { describe, expect, it } from 'vitest';
import { whyUnplayable } from './unplayable';

describe('saying why a film will not play', () => {
  it('names the drive when the file could not be read at all', () => {
    const said = whyUnplayable(2, '/films/Heat.1995.mkv');

    expect(said).toContain('could not be read');
    expect(said).toContain('disconnected');
  });

  it('separates a file that would not open from one that would not decode', () => {
    expect(whyUnplayable(4, '/films/Heat.1995.mkv')).toContain('cannot open this file');
    expect(whyUnplayable(3, '/films/Heat.1995.mkv')).toContain('could not be decoded');
  });

  it('guesses from the container, since that is all there is to guess from', () => {
    expect(whyUnplayable(3, '/films/Heat.1995.mkv')).toContain('HEVC video or DTS audio');
    expect(whyUnplayable(3, '/films/Heat.1995.mp4')).toContain('likely HEVC');
    expect(whyUnplayable(3, '/films/Heat.1995.AVI')).toContain('older codec');
    expect(whyUnplayable(3, '/films/Heat.1995.xyz')).toContain('no decoder for');
  });

  it('always says what would play instead', () => {
    for (const path of ['a.mkv', 'a.mp4', 'a.mov', 'a.avi', 'a.webm', 'a']) {
      expect(whyUnplayable(3, path)).toContain('H.264');
    }
  });

  it('promises no conversion it will not do', () => {
    expect(whyUnplayable(4, 'a.mkv')).toContain('does not convert');
  });

  it('says something useful when the element reports nothing', () => {
    expect(whyUnplayable(null, 'a.mkv')).toContain('did not say why');
    expect(whyUnplayable(undefined, 'a.mkv')).toContain('did not say why');
    expect(whyUnplayable(1, 'a.mkv')).toContain('stopped before it could play');
  });
});
