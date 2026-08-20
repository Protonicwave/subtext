import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';

// The URL a file is served from is the shell's business, and there is no shell
// under test. The tile only cares that it has one.
vi.mock('@/shared/media/source', () => ({ sourceOf: (path: string) => `asset://${path}` }));

const { PosterTile } = await import('./PosterTile');
const { useSettings } = await import('@/shared/settings/useSettings');
const { DEFAULTS } = await import('@/shared/settings/schema');

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

function show(film: FilmView) {
  return render(<PosterTile film={film} onOpen={() => undefined} />).container;
}

describe('a film in the grid', () => {
  beforeEach(() => {
    useSettings.setState({ settings: DEFAULTS });
  });

  it('draws the artwork found for a film', () => {
    const container = show({ ...film, coverSource: 'beside' });

    expect(container.querySelector('img')).toHaveAttribute(
      'src',
      'asset:///data/posters/heat.webp',
    );
  });

  /*
   * The frame is still taken and is still on the film's row. What changed is
   * that it is no longer what a cover is, so the tile draws the composition
   * instead and the picture is left to the places that want a wide one.
   */
  it('composes a film with no artwork rather than showing its frame', () => {
    const container = show(film);

    expect(container.querySelector('img')).toBeNull();
    // Twice: over the composed cover, and in the caption under the tile.
    expect(screen.getAllByText('Heat')).toHaveLength(2);
  });

  it('shows the frame to anybody who has asked for frames', () => {
    useSettings.setState({ settings: { ...DEFAULTS, withoutArtwork: 'frame' } });

    const container = show(film);

    expect(container.querySelector('img')).toHaveAttribute(
      'src',
      'asset:///data/posters/heat.webp',
    );
  });

  it('composes a film whose poster has not been drawn yet, frames or not', () => {
    useSettings.setState({ settings: { ...DEFAULTS, withoutArtwork: 'frame' } });

    const container = show({ ...film, posterPath: null });

    expect(container.querySelector('img')).toBeNull();
    expect(screen.getAllByText('Heat').length).toBeGreaterThan(0);
  });
});
