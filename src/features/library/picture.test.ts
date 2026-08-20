import { describe, expect, it } from 'vitest';
import type { CoverSourceView, FilmView } from '@/shared/ipc/bindings';
import { pictureFor } from './picture';

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Crime/Heat.1995.mkv',
  shelf: { name: 'Crime', path: '/films/Crime' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: 170 * 60_000,
  posterPath: '/data/posters/heat.webp',
  coverPath: null,
  coverSource: 'none',
  accent: null,
  missing: false,
  details: null,
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

/** Every source but the last, which is the one that says nobody chose anything. */
const CHOSEN: CoverSourceView[] = ['chosen', 'in-file', 'beside', 'sidecar', 'folder-above'];

describe('which picture a film is drawn with', () => {
  it('draws artwork as the cover it is, wherever it was found', () => {
    for (const source of CHOSEN) {
      const drawn = { ...film, coverSource: source };

      expect(pictureFor(drawn, 'composed')).toStrictEqual({
        kind: 'artwork',
        path: '/data/posters/heat.webp',
      });
      // The setting says what to do with a film that has no artwork, so it says
      // nothing at all about a film that has some.
      expect(pictureFor(drawn, 'frame')).toStrictEqual(pictureFor(drawn, 'composed'));
    }
  });

  it('composes a film with no artwork rather than showing the frame taken from it', () => {
    expect(pictureFor(film, 'composed')).toStrictEqual({ kind: 'composed' });
  });

  it('gives the frame back to anybody who asks for it', () => {
    expect(pictureFor(film, 'frame')).toStrictEqual({
      kind: 'frame',
      path: '/data/posters/heat.webp',
    });
  });

  it('composes a film whose poster has not been drawn yet, whatever was found for it', () => {
    for (const source of [...CHOSEN, 'none' as const]) {
      const waiting = { ...film, coverSource: source, posterPath: null };

      expect(pictureFor(waiting, 'composed')).toStrictEqual({ kind: 'composed' });
      expect(pictureFor(waiting, 'frame')).toStrictEqual({ kind: 'composed' });
    }
  });

  it('draws a film whose file has gone with whatever picture it still has', () => {
    // The poster is a file of ours and is still there when the film is not, so
    // a missing film keeps its cover and is dimmed rather than emptied.
    const gone = { ...film, coverSource: 'beside' as const, missing: true };

    expect(pictureFor(gone, 'composed')).toStrictEqual({
      kind: 'artwork',
      path: '/data/posters/heat.webp',
    });
  });
});
