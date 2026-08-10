import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { FilmView, FolderView, ScanProgressed } from '../shared/ipc/bindings';
import { App } from './App';

const { commands, listeners } = vi.hoisted(() => ({
  commands: {
    chooseFolder: vi.fn(),
    addFolder: vi.fn(),
    listFolders: vi.fn(),
    listLibrary: vi.fn(),
    rescan: vi.fn(),
  },
  // What the window has subscribed to, so a test can push an event at it.
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));

vi.mock('../shared/ipc/bindings', () => ({
  commands,
  events: {
    scanProgressed: { listen: listen('scan-progressed') },
    scanFinished: { listen: listen('scan-finished') },
    scanFailed: { listen: listen('scan-failed') },
  },
}));

function listen(name: string) {
  return (callback: (event: { payload: unknown }) => void) => {
    listeners.set(name, callback);
    return Promise.resolve(() => listeners.delete(name));
  };
}

function ok<T>(data: T) {
  return Promise.resolve({ status: 'ok' as const, data });
}

function failed(message: string) {
  return Promise.resolve({ status: 'error' as const, error: { message } });
}

const folder: FolderView = {
  id: 1,
  path: '/films',
  addedAt: 1_700_000_000_000,
  films: 2,
  watching: true,
};

const film: FilmView = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  title: 'Heat',
  year: 1995,
  durationMs: null,
  posterPath: null,
  accent: null,
  missing: false,
  tracks: [
    {
      id: 3,
      path: '/films/Heat.1995.srt',
      language: 'en',
      forced: false,
      hearingImpaired: false,
      matchKind: 'exact',
      cueCount: 1_402,
    },
  ],
  position: null,
};

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listeners.clear();
    commands.listFolders.mockReturnValue(ok([folder]));
    commands.listLibrary.mockReturnValue(ok([film]));
  });

  it('shows the folders being watched and what was found in them', async () => {
    render(<App />);

    expect(await screen.findByText(/\/films \(2 films, watching\)/)).toBeInTheDocument();
    expect(await screen.findByText(/Heat \(1995\), 1402 lines/)).toBeInTheDocument();
  });

  it('adds the folder that was chosen', async () => {
    commands.chooseFolder.mockReturnValue(ok('/more films'));
    commands.addFolder.mockReturnValue(ok({ ...folder, id: 2, path: '/more films' }));

    render(<App />);
    await userEvent.click(screen.getByRole('button', { name: 'Add a folder' }));

    await waitFor(() => {
      expect(commands.addFolder).toHaveBeenCalledWith('/more films');
    });
  });

  it('asks for nothing when the picker was dismissed', async () => {
    commands.chooseFolder.mockReturnValue(ok(null));

    render(<App />);
    await userEvent.click(screen.getByRole('button', { name: 'Add a folder' }));

    await waitFor(() => {
      expect(commands.chooseFolder).toHaveBeenCalled();
    });
    expect(commands.addFolder).not.toHaveBeenCalled();
  });

  it('follows a scan as it reports itself', async () => {
    render(<App />);
    await screen.findByText(/Heat/);

    const progress: ScanProgressed = {
      folderId: 1,
      stage: 'indexing',
      filesSeen: 40,
      filmsFound: 20,
      subtitlesFound: 20,
      filmsPaired: 20,
      subtitlesToRead: 20,
      subtitlesRead: 12,
      cuesIndexed: 9_000,
      fractionRead: 0.6,
    };
    listeners.get('scan-progressed')?.({ payload: progress });

    expect(
      await screen.findByText(/indexing: 20 films, 12 of 20 subtitle files read/),
    ).toBeInTheDocument();
  });

  it('says what went wrong rather than showing nothing', async () => {
    commands.listFolders.mockReturnValue(failed('the library database refused the request'));

    render(<App />);

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'the library database refused the request',
    );
  });
});
