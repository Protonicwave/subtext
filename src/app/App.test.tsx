import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, FolderView } from '@/shared/ipc/bindings';

const { ipc, drops } = vi.hoisted(() => ({
  ipc: {
    windowChrome: vi.fn(() => Promise.resolve({ backdrop: false })),
    listFolders: vi.fn((): Promise<FolderView[]> => Promise.resolve([])),
    listLibrary: vi.fn((): Promise<FilmView[]> => Promise.resolve([])),
    chooseFolder: vi.fn(),
  },
  drops: vi.fn(() => Promise.resolve(() => undefined)),
}));

const emitter = () => ({ listen: () => Promise.resolve(() => undefined) });

vi.mock('@/shared/ipc/bindings', () => ({
  commands: {},
  events: { scanProgressed: emitter(), scanFinished: emitter(), scanFailed: emitter() },
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
  title: 'Heat',
  year: 1995,
  durationMs: null,
  posterPath: null,
  accent: null,
  missing: false,
  tracks: [],
  position: null,
} satisfies FilmView;

describe('the application', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.documentElement.removeAttribute('data-backdrop');
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
    useLibrary.setState({ folders: [], films: [], loaded: false, problem: null });
    ipc.windowChrome.mockResolvedValue({ backdrop: false });
    ipc.listFolders.mockResolvedValue([]);
    ipc.listLibrary.mockResolvedValue([]);
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

    expect(await screen.findByText('Heat')).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /choose your films folder/i }),
    ).not.toBeInTheDocument();
  });

  it('moves between the screens and back again', async () => {
    ipc.listFolders.mockResolvedValue([folder]);
    ipc.listLibrary.mockResolvedValue([film]);
    render(<App />);
    await screen.findByText('Heat');

    await userEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(await screen.findByRole('heading', { name: /watched folders/i })).toBeInTheDocument();

    await userEvent.keyboard('{Escape}');
    expect(await screen.findByRole('heading', { name: /your films/i })).toBeInTheDocument();
  });

  it('opens a film, which is as far as the player goes for now', async () => {
    ipc.listFolders.mockResolvedValue([folder]);
    ipc.listLibrary.mockResolvedValue([film]);
    render(<App />);

    await userEvent.click(await screen.findByText('Heat'));

    expect(await screen.findByText(/playback itself is not built yet/i)).toBeInTheDocument();
  });

  it('marks the window when the desktop shows through behind it', async () => {
    ipc.windowChrome.mockResolvedValue({ backdrop: true });

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

  it('listens for files dropped anywhere in the window', () => {
    render(<App />);

    expect(drops).toHaveBeenCalled();
  });
});
