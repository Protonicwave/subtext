import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, FolderView } from '@/shared/ipc/bindings';

const { ipc, drops } = vi.hoisted(() => ({
  ipc: {
    windowChrome: vi.fn(() => Promise.resolve({ backdrop: false, switchableDecoding: false })),
    readPreferences: vi.fn(() => Promise.resolve([])),
    writePreference: vi.fn(() => Promise.resolve(null)),
    listFolders: vi.fn((): Promise<FolderView[]> => Promise.resolve([])),
    listLibrary: vi.fn((): Promise<FilmView[]> => Promise.resolve([])),
    continueWatching: vi.fn((): Promise<FilmView[]> => Promise.resolve([])),
    chooseFolder: vi.fn(),
  },
  drops: vi.fn(() => Promise.resolve(() => undefined)),
}));

const emitter = () => ({ listen: () => Promise.resolve(() => undefined) });

vi.mock('@/shared/ipc/bindings', () => ({
  commands: {},
  events: {
    scanProgressed: emitter(),
    scanFinished: emitter(),
    scanFailed: emitter(),
    alignProgressed: emitter(),
  },
}));
vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});
vi.mock('@/shared/window/drops', () => ({ onFilesDropped: drops }));
vi.mock('@/shared/window/controls', () => ({
  windowControls: {
    minimise: vi.fn(),
    toggleMaximise: vi.fn(),
    close: vi.fn(),
    isMaximised: vi.fn(() => Promise.resolve(false)),
    onResized: vi.fn(() => Promise.resolve(() => undefined)),
  },
}));

const { App } = await import('./App');
const { useNavigation } = await import('./routes');
const { useLibrary } = await import('@/features/library/useLibrary');

const folder = { id: 1, path: '/films', addedAt: 0, films: 1, watching: true } satisfies FolderView;

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  shelf: { name: 'films', path: '/films' },
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

describe('the application', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.documentElement.removeAttribute('data-backdrop');
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
    useLibrary.setState({ folders: [], films: [], resumable: [], loaded: false, problem: null });
    ipc.windowChrome.mockResolvedValue({ backdrop: false, switchableDecoding: false });
    ipc.listFolders.mockResolvedValue([]);
    ipc.listLibrary.mockResolvedValue([]);
    ipc.continueWatching.mockResolvedValue([]);
  });

  it('asks for a folder when nothing is being watched', async () => {
    render(<App />);

    expect(
      await screen.findByRole('button', { name: /choose your films folder/i }),
    ).toBeInTheDocument();
  });

  it('shows the library instead once a folder is being watched', async () => {
    ipc.listFolders.mockResolvedValue([folder]);
    ipc.listLibrary.mockResolvedValue([film]);

    render(<App />);

    expect(await screen.findByRole('button', { name: /Heat/ })).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /choose your films folder/i }),
    ).not.toBeInTheDocument();
  });

  it('moves between the screens and back again', async () => {
    ipc.listFolders.mockResolvedValue([folder]);
    ipc.listLibrary.mockResolvedValue([film]);
    render(<App />);
    await screen.findByRole('button', { name: /Heat/ });

    await userEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(await screen.findByRole('heading', { name: /watched folders/i })).toBeInTheDocument();

    await userEvent.keyboard('{Escape}');
    // The library screen is the one with a film shown large at the top of it.
    expect(await screen.findByRole('heading', { name: 'Heat' })).toBeInTheDocument();
  });

  it('opens a film’s page, and the film from it', async () => {
    ipc.listFolders.mockResolvedValue([folder]);
    ipc.listLibrary.mockResolvedValue([film]);
    render(<App />);

    await userEvent.click(await screen.findByRole('button', { name: /Heat/ }));
    const sheet = await screen.findByRole('dialog', { name: 'Heat' });

    await userEvent.click(within(sheet).getByRole('button', { name: 'Play' }));

    // Nothing decodes under test, so the film sits on its first frame with the
    // controls showing, which is the right answer to a file that has not
    // started: something to press.
    expect(await screen.findByRole('button', { name: 'Play' })).toBeInTheDocument();
    expect(document.querySelector('video')).not.toBeNull();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('marks the window when the desktop shows through behind it', async () => {
    ipc.windowChrome.mockResolvedValue({ backdrop: true, switchableDecoding: false });

    render(<App />);

    await waitFor(() => {
      expect(document.documentElement.dataset.backdrop).toBe('mica');
    });
  });

  it('leaves the window plain when it has no backdrop', async () => {
    render(<App />);

    await waitFor(() => {
      expect(ipc.windowChrome).toHaveBeenCalled();
    });
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it('says what went wrong when the library cannot be read', async () => {
    ipc.listFolders.mockRejectedValue(new Error('the library database refused the request'));

    render(<App />);

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'the library database refused the request',
    );
  });

  /*
   * None of these failures stops anything, so the strip has to be closable.
   * One that was not would sit over the window for the rest of the session
   * reporting something already read and dealt with.
   */
  it('puts a failure away when it is dismissed', async () => {
    ipc.listFolders.mockRejectedValue(new Error('the library database refused the request'));

    render(<App />);
    await screen.findByRole('alert');

    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('listens for files dropped anywhere in the window', () => {
    render(<App />);

    expect(drops).toHaveBeenCalled();
  });
});
