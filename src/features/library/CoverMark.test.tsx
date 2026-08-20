import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { CoverSourceView, FilmView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    chooseImage: vi.fn((): Promise<string | null> => Promise.resolve(null)),
    chooseCover: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
    clearCover: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { CoverMark } = await import('./CoverMark');
const { useLibrary } = await import('./useLibrary');

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

function show(source: CoverSourceView) {
  const showing = { ...film, coverSource: source };
  useLibrary.setState({ films: [showing], resumable: [], loaded: true });
  render(<CoverMark film={showing} shown />);
  return showing;
}

describe('the mark on a tile', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /*
   * The mark is the film page's statement in the shortest form that still says
   * it, so every source the back end can send has to have one.
   */
  it('names each of the six sources', () => {
    const named: [CoverSourceView, string][] = [
      ['chosen', 'Your choice'],
      ['in-file', 'Inside the film'],
      ['beside', 'Beside the film'],
      ['sidecar', 'From a media manager'],
      ['folder-above', 'From the folder above'],
      ['none', 'From the film itself'],
    ];

    for (const [source, name] of named) {
      const { unmount } = render(<CoverMark film={{ ...film, coverSource: source }} shown />);
      expect(screen.getByRole('button', { name })).toBeInTheDocument();
      unmount();
    }
  });

  it('keeps the actions behind the mark until it is opened', async () => {
    show('none');

    expect(screen.queryByRole('button', { name: 'Choose an image' })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'From the film itself' }));

    expect(screen.getByRole('button', { name: 'Choose an image' })).toBeInTheDocument();
  });

  it('settles a picture through the same command the film page uses', async () => {
    ipc.chooseImage.mockReturnValue(Promise.resolve('/pictures/Heat.png'));
    ipc.chooseCover.mockReturnValue(Promise.resolve({ ...film, coverSource: 'chosen' as const }));
    show('none');

    await userEvent.click(screen.getByRole('button', { name: 'From the film itself' }));
    await userEvent.click(screen.getByRole('button', { name: 'Choose an image' }));

    await waitFor(() => {
      expect(useLibrary.getState().films[0]?.coverSource).toBe('chosen');
    });
    expect(ipc.chooseCover).toHaveBeenCalledWith(film.id, '/pictures/Heat.png');
  });

  it('offers to put a choice back only where there is one', async () => {
    show('beside');
    await userEvent.click(screen.getByRole('button', { name: 'Beside the film' }));

    expect(screen.queryByRole('button', { name: 'Use what was found' })).not.toBeInTheDocument();

    show('chosen');
    await userEvent.click(screen.getByRole('button', { name: 'Your choice' }));

    expect(screen.getByRole('button', { name: 'Use what was found' })).toBeInTheDocument();
  });

  it('gives the film back to the scan', async () => {
    ipc.clearCover.mockReturnValue(Promise.resolve({ ...film, coverSource: 'none' as const }));
    show('chosen');

    await userEvent.click(screen.getByRole('button', { name: 'Your choice' }));
    await userEvent.click(screen.getByRole('button', { name: 'Use what was found' }));

    await waitFor(() => {
      expect(ipc.clearCover).toHaveBeenCalledWith(film.id);
    });
    expect(useLibrary.getState().films[0]?.coverSource).toBe('none');
  });

  /*
   * Escape closes the mark and goes no further. Left to travel, it would also
   * be read as leaving the screen behind the tile, which is what it means
   * everywhere else on this page.
   */
  it('closes on Escape without the grid behind it hearing it', async () => {
    const heard = vi.fn();
    render(
      <div onKeyDown={heard}>
        <CoverMark film={film} shown />
      </div>,
    );

    await userEvent.click(screen.getByRole('button', { name: 'From the film itself' }));
    await userEvent.keyboard('{Escape}');

    expect(screen.queryByRole('button', { name: 'Choose an image' })).not.toBeInTheDocument();
    expect(heard).not.toHaveBeenCalled();
  });
});
