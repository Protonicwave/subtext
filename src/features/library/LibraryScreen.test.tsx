import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, FolderView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: { chooseFolder: vi.fn(), postersWanted: vi.fn(() => Promise.resolve([])) },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});
// The URL a file is served from is the shell's business, and there is no shell
// under test. The screen only cares that it has one.
vi.mock('@/shared/media/source', () => ({ sourceOf: (path: string) => `asset://${path}` }));

const { LibraryScreen } = await import('./LibraryScreen');
const { useLibrary } = await import('./useLibrary');
const { useNavigation } = await import('@/app/routes');

const folder = { id: 1, path: '/films', addedAt: 0, films: 2, watching: true } satisfies FolderView;

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  title: 'Heat',
  year: 1995,
  durationMs: 170 * 60_000,
  posterPath: null,
  accent: null,
  missing: false,
  details: null,
  tracks: [
    {
      id: 3,
      path: '/films/Heat.1995.srt',
      language: 'en',
      forced: false,
      hearingImpaired: false,
      origin: 'sidecar' as const,
      streamNumber: 0,
      form: 'text' as const,
      matchKind: 'exact' as const,
      correction: { offsetMs: 0, rate: 1 },
      cueCount: 1_402,
    },
  ],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

const watching = {
  ...film,
  id: 8,
  title: 'Ronin',
  position: {
    positionMs: 122 * 60_000,
    durationMs: 170 * 60_000,
    finished: false,
    updatedAt: 5,
    progress: 122 / 170,
  },
} satisfies FilmView;

/** Read, unless a test is about what the screen says before the read comes back. */
function show(
  state: Partial<{
    films: FilmView[];
    resumable: FilmView[];
    folders: FolderView[];
    loaded: boolean;
  }>,
) {
  useLibrary.setState({ folders: [folder], films: [], resumable: [], loaded: true, ...state });
  render(<LibraryScreen />);
}

describe('the library screen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
  });

  it('says what is there, and how much dialogue it has', () => {
    show({ films: [film] });

    expect(screen.getByRole('button', { name: /Heat/ })).toBeInTheDocument();
    expect(screen.getByText(/1 film · 1,402 lines of dialogue/)).toBeInTheDocument();
  });

  it('opens a film when its tile is chosen', async () => {
    show({ films: [film] });

    await userEvent.click(screen.getByRole('button', { name: /Heat/ }));

    expect(useNavigation.getState().route).toMatchObject({ screen: 'player', filmId: 7 });
  });

  it('offers what there is to carry on with, and how much of it is left', () => {
    show({ films: [film, watching], resumable: [watching] });

    expect(screen.getByRole('heading', { name: /carry on watching/i })).toBeInTheDocument();
    expect(screen.getByText('48 min left')).toBeInTheDocument();
  });

  it('says nothing about carrying on when there is nothing to carry on with', () => {
    show({ films: [film] });

    expect(screen.queryByRole('heading', { name: /carry on watching/i })).not.toBeInTheDocument();
  });

  it('shows a film whose file has gone as missing rather than hiding it', () => {
    show({ films: [{ ...film, missing: true }] });

    expect(screen.getByRole('button', { name: /Heat/ })).toBeInTheDocument();
    expect(screen.getAllByText(/missing/i).length).toBeGreaterThan(0);
  });

  it('draws a film that has a captured frame with it', () => {
    show({ films: [{ ...film, posterPath: '/data/posters/abc.webp' }] });

    // Decorative, so it has no accessible name to find it by: the title beside
    // it is what says which film this is.
    expect(document.querySelector('img')).toHaveAttribute('src', 'asset:///data/posters/abc.webp');
  });

  it('draws a handful of tiles for a library of ten thousand films', () => {
    const many = Array.from({ length: 10_000 }, (_, at) => ({
      ...film,
      id: at + 1,
      title: `Film ${String(at + 1)}`,
    }));

    const at = performance.now();
    show({ films: many });
    const took = performance.now() - at;

    // The number itself is the window's worth plus the overscan, and it does
    // not move with the size of the library. That is the whole property the
    // scrolling target rests on: a row costs the same whether there are ten
    // rows behind it or ten thousand.
    const tiles = document.querySelectorAll('button').length;
    expect(tiles).toBeLessThan(60);
    // Found by the tile's caption rather than by its text, since the cover
    // composed for a film with no picture sets the title as well.
    expect(screen.getByRole('button', { name: 'Film 11995 · 1,402 lines' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Film 9000/ })).not.toBeInTheDocument();

    // Slow enough to catch a return to drawing all of them, and loose enough
    // not to fail on a busy machine.
    expect(took).toBeLessThan(2_000);
  });

  it('tells somebody with no films which of the two reasons it is', () => {
    show({ films: [], folders: [] });
    expect(screen.getByText(/no folders are being watched yet/i)).toBeInTheDocument();
  });

  it('says so when a watched folder turned out to hold nothing', () => {
    show({ films: [], folders: [folder] });
    expect(screen.getByText(/nothing was found in the folders/i)).toBeInTheDocument();
  });

  /*
   * The difference between a library that is empty and one that has not been
   * read yet. Both have no films in hand, and only one of them is worth telling
   * somebody about.
   */
  it('claims nothing about what is on disk until the first read has come back', () => {
    show({ films: [], folders: [], loaded: false });

    expect(screen.getByRole('status')).toHaveTextContent(/reading your films/i);
    expect(screen.queryByText(/no folders are being watched yet/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/nothing was found/i)).not.toBeInTheDocument();
  });
});
