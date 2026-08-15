import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, FolderView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    chooseFolder: vi.fn(),
    postersWanted: vi.fn(() => Promise.resolve([])),
    writePreference: vi.fn(() => Promise.resolve(null)),
  },
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
const { useSheet } = await import('./useSheet');
const { useNavigation } = await import('@/app/routes');
const { useSettings } = await import('@/shared/settings/useSettings');
const { DEFAULTS } = await import('@/shared/settings/schema');

const folder = { id: 1, path: '/films', addedAt: 0, films: 2, watching: true } satisfies FolderView;

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

/** A film on a shelf of its own, since which folder a film is in is now a fact. */
function on(shelf: string, changes: Partial<FilmView> = {}): FilmView {
  return { ...film, shelf: { name: shelf, path: `/films/${shelf}` }, ...changes };
}

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

/** The rows, in the order they are drawn. */
function shelfNames(): string[] {
  return screen.getAllByRole('heading', { level: 2 }).map((heading) => heading.textContent);
}

describe('the library screen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSheet.setState({ filmId: null });
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
    useSettings.setState({ settings: DEFAULTS });
  });

  it('says what is there', () => {
    show({ films: [film] });

    expect(screen.getByRole('button', { name: /Heat/ })).toBeInTheDocument();
    expect(screen.getByText('1 film · 1 shelf')).toBeInTheDocument();
  });

  /*
   * A tile opens the film's page rather than the film. What a file is turns out
   * to be the question somebody has while looking at a wall of covers, and the
   * page answers it with the film one press away.
   */
  it('opens a film’s page when its tile is chosen', async () => {
    show({ films: [film] });

    await userEvent.click(screen.getByRole('button', { name: /Heat/ }));

    expect(useSheet.getState().filmId).toBe(7);
    expect(useNavigation.getState().route).toMatchObject({ screen: 'library' });
  });

  it('opens the same page from a row of the list', async () => {
    useSettings.setState({ settings: { ...DEFAULTS, libraryView: 'list' } });
    show({ films: [film] });

    await userEvent.click(screen.getByRole('button', { name: 'Heat' }));

    expect(useSheet.getState().filmId).toBe(7);
  });

  /*
   * The billboard is the exception. It is already showing one film large, so it
   * offers to play it and puts the page beside that rather than in front of it.
   */
  it('plays the film on the billboard, and offers its page beside it', async () => {
    show({ films: [film, watching], resumable: [watching] });

    await userEvent.click(screen.getByRole('button', { name: /carry on$/i }));
    expect(useNavigation.getState().route).toMatchObject({ screen: 'player', filmId: 8 });

    await userEvent.click(screen.getByRole('button', { name: 'Details' }));
    expect(useSheet.getState().filmId).toBe(8);
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

  it('makes a row of each folder, named and with its path beside it', () => {
    show({ films: [on('Crime'), on('Epics', { id: 9, title: 'Stalker' })] });

    expect(shelfNames()).toEqual(['Crime', 'Epics']);
    expect(screen.getByText('/films/Crime')).toBeInTheDocument();
    expect(screen.getByText('/films/Epics')).toBeInTheDocument();
  });

  /*
   * The library arrives in alphabetical order, and the rows do not follow it:
   * they follow the order the folders were first met on disk, which is what
   * makes them the same rows in the same places between runs.
   */
  it('puts the rows in the order the folders were first met on disk', () => {
    show({
      films: [on('Crime', { id: 9 }), on('Epics', { id: 2, title: 'Stalker' })],
    });

    expect(shelfNames()).toEqual(['Epics', 'Crime']);
  });

  it('puts what there is to carry on with above the folders', () => {
    show({ films: [on('Crime'), watching], resumable: [watching] });

    expect(shelfNames()).toEqual(['Carry on watching', 'Crime', 'films']);
  });

  it('shows the film to carry on with large, above the rows', () => {
    show({ films: [film, watching], resumable: [watching] });

    expect(screen.getByRole('heading', { level: 1, name: 'Ronin' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /carry on/i })).toBeInTheDocument();
  });

  /*
   * A library nobody has watched yet still has something to show, and what
   * arrived last is the likeliest reason the application was opened.
   */
  it('shows the most recently added film large when nothing has been started', () => {
    show({ films: [film, on('Crime', { id: 40, title: 'Stalker' })] });

    expect(screen.getByRole('heading', { level: 1, name: 'Stalker' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^play$/i })).toBeInTheDocument();
  });

  it('gives back the one wall when the shelves are turned off', () => {
    useSettings.setState({ settings: { ...DEFAULTS, libraryLayout: 'wall' } });
    show({ films: [on('Crime'), on('Epics', { id: 9, title: 'Stalker' })] });

    expect(screen.queryByText('/films/Crime')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Heat/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Stalker/ })).toBeInTheDocument();
  });

  /*
   * The list serves a library the wall has stopped serving, so it is a second
   * view over the same films rather than a screen of its own.
   */
  it('gives the list instead of the covers when that is the view chosen', () => {
    useSettings.setState({ settings: { ...DEFAULTS, libraryView: 'list' } });
    show({ films: [on('Crime'), on('Epics', { id: 9, title: 'Stalker' })] });

    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Heat' })).toBeInTheDocument();
    expect(screen.queryAllByRole('heading', { level: 2 })).toEqual([]);
  });

  it('remembers which of the two views somebody asked for', async () => {
    show({ films: [film] });

    await userEvent.click(screen.getByRole('button', { name: 'List' }));
    expect(useSettings.getState().settings.libraryView).toBe('list');
    expect(screen.getByRole('table')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Covers' }));
    expect(useSettings.getState().settings.libraryView).toBe('covers');
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
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

  it('draws a handful of tiles along a row of ten thousand films', () => {
    const many = Array.from({ length: 10_000 }, (_, at) => ({
      ...film,
      id: at + 1,
      title: `Film ${String(at + 1)}`,
    }));

    show({ films: many });

    // A rail holds only the tiles that fit across it and a few either side,
    // however many films the folder turned out to have in it.
    expect(document.querySelectorAll('button').length).toBeLessThan(60);
    expect(screen.getByRole('button', { name: 'Film 11995 · 1,402 lines' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Film 9000/ })).not.toBeInTheDocument();
  });

  it('draws a handful of tiles for a wall of ten thousand films', () => {
    useSettings.setState({ settings: { ...DEFAULTS, libraryLayout: 'wall' } });
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
