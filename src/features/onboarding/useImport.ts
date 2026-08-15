import { create } from 'zustand';
import type { Id, ScanProgressed, ScanSummary } from '@/shared/ipc/bindings';
import { ipc, reasonFor } from '@/shared/ipc/client';
import { useLibrary } from '@/features/library/useLibrary';

/**
 * Where an import has got to.
 *
 * `indexing` while the scan runs, `results` once it has finished and there is
 * something to show. Idle covers both never having started and having been
 * dismissed, since neither puts anything on the screen.
 */
export type Stage = 'idle' | 'indexing' | 'results';

interface ImportState {
  stage: Stage;
  progress: ScanProgressed | null;
  summaries: ScanSummary[];
  problem: string | null;
  /**
   * Whether a person asked for the scan that is running.
   *
   * Subtext rescans every watched folder when it starts, and folder watching
   * starts one whenever a file lands in one. Neither should throw a sheet up in
   * front of somebody who was doing something else, so only a scan that was
   * asked for opens one.
   */
  asked: boolean;

  /** Opens the folder picker, and adds what was chosen. */
  chooseFolder: () => Promise<void>;
  /** Adds folders that were dropped on the window. */
  addDropped: (paths: string[]) => Promise<void>;
  /** Reads every watched folder afresh. */
  reread: () => Promise<void>;
  saw: (progress: ScanProgressed) => void;
  finished: (summaries: ScanSummary[]) => void;
  failed: (message: string) => void;
  dismiss: () => void;
}

export const useImport = create<ImportState>((set, get) => ({
  stage: 'idle',
  progress: null,
  summaries: [],
  problem: null,
  asked: false,

  chooseFolder: async () => {
    try {
      const chosen = await ipc.chooseFolder();
      if (chosen === null) return;
      await begin(set, [chosen]);
    } catch (failure) {
      set({ problem: reasonFor(failure), stage: 'idle', asked: false });
    }
  },

  addDropped: async (paths) => {
    try {
      // Which folders those paths stand for is a filesystem question, so Rust
      // answers it: a dropped film means the folder it lives in.
      const folders = await ipc.foldersForPaths(paths);
      if (folders.length === 0) {
        set({ problem: 'Those are not films or subtitle files.' });
        return;
      }
      await begin(set, folders);
    } catch (failure) {
      set({ problem: reasonFor(failure), stage: 'idle', asked: false });
    }
  },

  /*
   * A scan of everything, so it says where it has got to the same way an import
   * does and ends in the same summary. Asked for, and therefore shown: somebody
   * who pressed the button is waiting to hear that it worked.
   */
  reread: async () => {
    set({ stage: 'indexing', asked: true, problem: null, progress: null, summaries: [] });
    try {
      await ipc.rescan();
    } catch (failure) {
      set({ problem: reasonFor(failure), stage: 'idle', asked: false });
    }
  },

  saw: (progress) => {
    // A scan nobody asked for still moves the progress along, for anything that
    // wants to show it quietly, but it does not open the sheet.
    set((state) => (state.asked ? { progress, stage: 'indexing' } : { progress }));
  },

  finished: (summaries) => {
    void useLibrary.getState().refresh();
    if (!get().asked) {
      set({ progress: null });
      return;
    }
    set({ stage: 'results', summaries, progress: null, asked: false });
  },

  failed: (message) => {
    set({ problem: message, stage: 'idle', progress: null, asked: false });
  },

  dismiss: () => {
    set({ stage: 'idle', summaries: [], problem: null });
  },
}));

/**
 * Records the folders and waits for the scan each one starts.
 *
 * The sheet is shown before anything has been read, because adding a folder
 * returns as soon as it has been written down and a thousand films take a few
 * seconds after that. An empty progress view is the honest thing to show in
 * between.
 */
async function begin(set: (state: Partial<ImportState>) => void, folders: string[]) {
  set({ stage: 'indexing', asked: true, problem: null, progress: null, summaries: [] });
  for (const folder of folders) {
    await useLibrary.getState().addFolder(folder);
  }
}

/** The folders a set of scan summaries covered. */
export function foldersIn(summaries: readonly ScanSummary[]): Id[] {
  return summaries.map((summary) => summary.folderId);
}

/** Everything the scans found that belongs to no film at all. */
export function unpairedIn(summaries: readonly ScanSummary[]): string[] {
  return summaries.flatMap((summary) => summary.unpairedSubtitles);
}
