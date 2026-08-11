import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, FolderView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    listFolders: vi.fn(),
    listLibrary: vi.fn(),
    continueWatching: vi.fn(),
    addFolder: vi.fn(),
    removeFolder: vi.fn(),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { fileNameOf, filmsIn, linesOf, useLibrary } = await import('./useLibrary');

const folder = { id: 1, path: '/films', addedAt: 0, films: 2, watching: true } satisfies FolderView;

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
  tracks: [
    {
      id: 3,
      path: '/films/Heat.1995.srt',
      language: 'en',
      forced: false,
      hearingImpaired: false,
      matchKind: 'exact' as const,
      cueCount: 1_402,
    },
  ],
  position: null,
} satisfies FilmView;

describe('the library', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useLibrary.setState({ folders: [], films: [], resumable: [], loaded: false, problem: null });
    ipc.listFolders.mockResolvedValue([folder]);
    ipc.listLibrary.mockResolvedValue([film]);
    ipc.continueWatching.mockResolvedValue([]);
  });

  it('reads the folders, the films and what to carry on with together', async () => {
    ipc.continueWatching.mockResolvedValue([film]);

    await useLibrary.getState().refresh();

    expect(useLibrary.getState().folders).toEqual([folder]);
    expect(useLibrary.getState().films).toEqual([film]);
    expect(useLibrary.getState().resumable).toEqual([film]);
    expect(useLibrary.getState().loaded).toBe(true);
  });

  it('says what went wrong rather than showing an empty library', async () => {
    ipc.listFolders.mockRejectedValue(new Error('the library database refused the request'));

    await useLibrary.getState().refresh();

    expect(useLibrary.getState().problem).toBe('the library database refused the request');
    // Loaded either way, so the screen shows the failure instead of waiting.
    expect(useLibrary.getState().loaded).toBe(true);
  });

  it('shows a folder as soon as it has been added, before it has been read', async () => {
    ipc.addFolder.mockResolvedValue(folder);

    await useLibrary.getState().addFolder('/films');

    expect(useLibrary.getState().folders).toEqual([folder]);
  });

  it('does not list the same folder twice', async () => {
    ipc.addFolder.mockResolvedValue(folder);

    await useLibrary.getState().addFolder('/films');
    await useLibrary.getState().addFolder('/films');

    expect(useLibrary.getState().folders).toHaveLength(1);
  });

  it('takes the films with the folder they were found in', async () => {
    useLibrary.setState({ folders: [folder], films: [film], resumable: [film] });
    ipc.removeFolder.mockResolvedValue(true);

    await useLibrary.getState().removeFolder(1);

    expect(useLibrary.getState().folders).toEqual([]);
    expect(useLibrary.getState().films).toEqual([]);
    expect(useLibrary.getState().resumable).toEqual([]);
  });

  it('puts back the one film a capture changed, and leaves the rest alone', () => {
    const other = { ...film, id: 9, title: 'Ronin' };
    useLibrary.setState({ films: [film, other], resumable: [film] });

    const captured = { ...film, posterPath: '/data/posters/abc.webp' };
    useLibrary.getState().replace(captured);

    expect(useLibrary.getState().films).toEqual([captured, other]);
    // Wherever else the same film is being shown, it says the same thing.
    expect(useLibrary.getState().resumable).toEqual([captured]);
  });

  it('has nothing to put back for a film that is no longer listed', () => {
    useLibrary.setState({ films: [film], resumable: [] });

    useLibrary.getState().replace({ ...film, id: 404 });

    expect(useLibrary.getState().films).toEqual([film]);
  });

  it('counts the lines across every subtitle track a film has', () => {
    expect(linesOf(film)).toBe(1_402);
    expect(linesOf({ ...film, tracks: [] })).toBe(0);
  });

  it('picks out the films found in a set of folders', () => {
    const elsewhere = { ...film, id: 9, folderId: 2 };

    expect(filmsIn([film, elsewhere], [1])).toEqual([film]);
    expect(filmsIn([film, elsewhere], [1, 2])).toHaveLength(2);
    expect(filmsIn([film, elsewhere], [])).toEqual([]);
  });

  it('finds the file name whichever way the platform writes a path', () => {
    expect(fileNameOf('/films/Heat.1995.mkv')).toBe('Heat.1995.mkv');
    expect(fileNameOf('D:\\Films\\Heat.1995.mkv')).toBe('Heat.1995.mkv');
    expect(fileNameOf('Heat.1995.mkv')).toBe('Heat.1995.mkv');
  });
});
