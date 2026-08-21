import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { FrameCover } from './FrameCover';

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

describe('a frame drawn where a cover goes', () => {
  it('draws the whole frame, and the same frame behind it to fill the tile', () => {
    // Two uses of one URL rather than a second picture: the shot at its own
    // shape, and the blurred extension of itself it sits on.
    const { container } = render(<FrameCover film={film} src="asset://heat.webp" />);
    const drawn = [...container.querySelectorAll('img')];

    expect(drawn).toHaveLength(2);
    expect(drawn.every((image) => image.getAttribute('src') === 'asset://heat.webp')).toBe(true);
  });

  it('sets the title over it, which a frame does not carry for itself', () => {
    render(<FrameCover film={film} src="asset://heat.webp" />);

    expect(screen.getByText('Heat')).toBeInTheDocument();
  });
});
