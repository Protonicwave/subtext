import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { ScanProgressed, ScanSummary } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    chooseFolder: vi.fn(),
    foldersForPaths: vi.fn(),
    addFolder: vi.fn(),
    listFolders: vi.fn(() => Promise.resolve([])),
    listLibrary: vi.fn(() => Promise.resolve([])),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { useImport } = await import('./useImport');
const { useLibrary } = await import('@/features/library/useLibrary');

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

const summary: ScanSummary = {
  folderId: 1,
  filmsFound: 20,
  subtitlesFound: 20,
  filmsPaired: 20,
  subtitlesRead: 20,
  cuesIndexed: 15_000,
  filmsMissing: 0,
  unpairedSubtitles: [],
  filmsWithoutSubtitles: [],
  unreadable: [],
  warnings: [],
};

describe('an import', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useImport.setState({
      stage: 'idle',
      progress: null,
      summaries: [],
      problem: null,
      asked: false,
    });
    useLibrary.setState({ folders: [], films: [] });
    ipc.addFolder.mockResolvedValue({
      id: 1,
      path: '/films',
      addedAt: 0,
      films: 0,
      watching: true,
    });
  });

  it('shows the folder being read as soon as it has been chosen', async () => {
    ipc.chooseFolder.mockResolvedValue('/films');

    await useImport.getState().chooseFolder();

    expect(ipc.addFolder).toHaveBeenCalledWith('/films');
    expect(useImport.getState().stage).toBe('indexing');
  });

  it('does nothing when the picker was dismissed', async () => {
    ipc.chooseFolder.mockResolvedValue(null);

    await useImport.getState().chooseFolder();

    expect(ipc.addFolder).not.toHaveBeenCalled();
    expect(useImport.getState().stage).toBe('idle');
  });

  it('adds the folder that dropped files belong to', async () => {
    ipc.foldersForPaths.mockResolvedValue(['/films']);

    await useImport.getState().addDropped(['/films/Heat.1995.mkv', '/films/Heat.1995.srt']);

    expect(ipc.foldersForPaths).toHaveBeenCalledWith([
      '/films/Heat.1995.mkv',
      '/films/Heat.1995.srt',
    ]);
    expect(ipc.addFolder).toHaveBeenCalledWith('/films');
  });

  it('says so when what was dropped was not films', async () => {
    ipc.foldersForPaths.mockResolvedValue([]);

    await useImport.getState().addDropped(['/documents/notes.txt']);

    expect(ipc.addFolder).not.toHaveBeenCalled();
    expect(useImport.getState().problem).toMatch(/not films/i);
  });

  it('shows what was found once the scan has finished', async () => {
    ipc.chooseFolder.mockResolvedValue('/films');
    await useImport.getState().chooseFolder();

    useImport.getState().saw(progress);
    expect(useImport.getState().progress).toEqual(progress);

    useImport.getState().finished([summary]);

    expect(useImport.getState().stage).toBe('results');
    expect(useImport.getState().summaries).toEqual([summary]);
  });

  it('leaves a scan nobody asked for to run quietly', () => {
    // What starting up and what a watched folder changing both produce.
    useImport.getState().saw(progress);
    useImport.getState().finished([summary]);

    expect(useImport.getState().stage).toBe('idle');
    expect(useImport.getState().summaries).toEqual([]);
  });

  it('reads the library again after any scan, asked for or not', () => {
    const refresh = vi.fn(() => Promise.resolve());
    useLibrary.setState({ refresh });

    useImport.getState().finished([summary]);

    expect(refresh).toHaveBeenCalled();
  });

  it('stops waiting when a scan fails', async () => {
    ipc.chooseFolder.mockResolvedValue('/films');
    await useImport.getState().chooseFolder();

    useImport.getState().failed('the drive was unplugged');

    expect(useImport.getState().stage).toBe('idle');
    expect(useImport.getState().problem).toBe('the drive was unplugged');
  });
});
