import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { ComposedCover } from './ComposedCover';
import { factsOf } from './runtime';

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  shelf: { name: 'films', path: '/films' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: 170 * 60_000,
  posterPath: null,
  coverSource: 'none',
  accent: null,
  missing: false,
  details: null,
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

describe('the cover composed for a film with no picture', () => {
  it('sets the title and what is known about the film under it', () => {
    render(<ComposedCover film={film} />);

    expect(screen.getByText('Heat')).toBeInTheDocument();
    expect(screen.getByText('1995 · 2 hr 50 min')).toBeInTheDocument();
  });

  it('leaves out a fact it does not have rather than showing it empty', () => {
    expect(factsOf({ ...film, durationMs: null })).toBe('1995');
    expect(factsOf({ ...film, year: null })).toBe('2 hr 50 min');
    expect(factsOf({ ...film, year: null, durationMs: null })).toBeNull();
  });

  it('draws a film nobody has opened yet as its title alone', () => {
    render(<ComposedCover film={{ ...film, year: null, durationMs: null }} />);

    expect(screen.getByText('Heat')).toBeInTheDocument();
    expect(screen.queryByText('·')).not.toBeInTheDocument();
  });
});
