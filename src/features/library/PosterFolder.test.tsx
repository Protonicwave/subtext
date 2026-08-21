import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { CoversTaken, FilmView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    chooseFolder: vi.fn((): Promise<string | null> => Promise.resolve(null)),
    coversFromFolder: vi.fn((): Promise<CoversTaken> =>
      Promise.resolve({ matched: 0, unmatched: 0 }),
    ),
    chooseImage: vi.fn((): Promise<string | null> => Promise.resolve(null)),
    chooseCover: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
    clearCover: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { PosterFolder } = await import('./PosterFolder');
const { useLibrary } = await import('./useLibrary');
const { IpcError } = await import('@/shared/ipc/client');

const film = {
  id: 1,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  shelf: { name: 'Films', path: '/films' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: null,
  posterPath: null,
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

/**
 * The library as it reads after the folder has been matched, which is what the
 * report counts. The read itself belongs to the library store and is stood in
 * for here.
 */
function reads(films: FilmView[]) {
  const refresh = vi.fn(() => {
    useLibrary.setState({ films });
    return Promise.resolve();
  });
  useLibrary.setState({ films: [film], refresh });
  return refresh;
}

describe('covering a library from a folder of pictures', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('matches the folder and says what came of it', async () => {
    const refresh = reads([{ ...film, coverSource: 'chosen', coverPath: '/posters/Heat.jpg' }]);
    ipc.chooseFolder.mockResolvedValue('/posters');
    ipc.coversFromFolder.mockResolvedValue({ matched: 1, unmatched: 0 });

    render(<PosterFolder />);
    await userEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));

    const sheet = await screen.findByRole('dialog', { name: 'Where your covers came from' });
    expect(ipc.coversFromFolder).toHaveBeenCalledWith('/posters');
    // Read back before anything is stated, so the tally describes the rows
    // rather than what the call hoped they would say.
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(sheet).toHaveTextContent('1 film took a cover from that folder.');
    expect(screen.getByText('/posters/Heat.jpg')).toBeInTheDocument();
  });

  it('says how many films the folder had nothing for', async () => {
    reads([film]);
    ipc.chooseFolder.mockResolvedValue('/posters');
    ipc.coversFromFolder.mockResolvedValue({ matched: 3, unmatched: 2 });

    render(<PosterFolder />);
    await userEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));

    const sheet = await screen.findByRole('dialog', { name: 'Where your covers came from' });
    expect(sheet).toHaveTextContent('3 films took a cover from that folder. The other 2 are as');
    // And the film with nothing is offered the way to settle it by hand.
    expect(screen.getByRole('button', { name: 'Choose a picture' })).toBeInTheDocument();
  });

  it('does nothing at all when no folder is chosen', async () => {
    reads([film]);
    ipc.chooseFolder.mockResolvedValue(null);

    render(<PosterFolder />);
    await userEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));

    expect(ipc.coversFromFolder).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('says what stopped it in the sentence the back end wrote', async () => {
    reads([film]);
    ipc.chooseFolder.mockResolvedValue('/posters');
    ipc.coversFromFolder.mockRejectedValue(new IpcError('those pictures could not be read'));

    render(<PosterFolder />);
    await userEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('those pictures could not be read');
    });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
