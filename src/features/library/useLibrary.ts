import { create } from 'zustand';
import type { FilmView, FolderView, Id } from '@/shared/ipc/bindings';
import { ipc, reasonFor } from '@/shared/ipc/client';

/**
 * What the library holds, as the screens see it.
 *
 * The database is the truth and this is a copy of it, so every action here ends
 * by reading it again rather than by patching the copy. A scan can change rows
 * nobody asked about, which makes guessing at the new state a way to be subtly
 * wrong.
 */
/** How many films the row above the grid offers to carry on with. */
const CONTINUE = 12;

interface LibraryState {
  folders: FolderView[];
  films: FilmView[];
  /** What to carry on with, most recently watched first. */
  resumable: FilmView[];
  /** False until the first read has come back, so the shell can wait. */
  loaded: boolean;
  problem: string | null;
  refresh: () => Promise<void>;
  addFolder: (path: string) => Promise<void>;
  removeFolder: (id: Id) => Promise<void>;
  /** Puts back one film that the back end has just changed. */
  replace: (film: FilmView) => void;
  clearProblem: () => void;
}

export const useLibrary = create<LibraryState>((set) => ({
  folders: [],
  films: [],
  resumable: [],
  loaded: false,
  problem: null,

  refresh: async () => {
    try {
      const [folders, films, resumable] = await Promise.all([
        ipc.listFolders(),
        ipc.listLibrary(),
        ipc.continueWatching(CONTINUE),
      ]);
      set({ folders, films, resumable, loaded: true, problem: null });
    } catch (failure) {
      set({ loaded: true, problem: reasonFor(failure) });
    }
  },

  addFolder: async (path) => {
    // Adding a folder records it and returns; what is inside it arrives through
    // the scan events, so this only has the folder itself to show for now.
    const folder = await ipc.addFolder(path);
    set((library) => ({
      folders: library.folders.some((known) => known.id === folder.id)
        ? library.folders
        : [...library.folders, folder],
      problem: null,
    }));
  },

  removeFolder: async (id) => {
    try {
      await ipc.removeFolder(id);
      set((library) => ({
        folders: library.folders.filter((folder) => folder.id !== id),
        films: library.films.filter((film) => film.folderId !== id),
        resumable: library.resumable.filter((film) => film.folderId !== id),
        problem: null,
      }));
    } catch (failure) {
      set({ problem: reasonFor(failure) });
    }
  },

  /*
   * The one place a film is patched rather than read back wholesale. Capturing
   * a poster changes one row and returns what that row now says, and re-reading
   * ten thousand films to learn it would undo the point of doing the captures
   * quietly in the background.
   */
  replace: (film) => {
    const swap = (films: FilmView[]) => films.map((known) => (known.id === film.id ? film : known));

    set((library) => ({
      films: swap(library.films),
      resumable: swap(library.resumable),
    }));
  },

  clearProblem: () => {
    set({ problem: null });
  },
}));

/** The films found inside a set of folders, in the order the library lists them. */
export function filmsIn(films: FilmView[], folderIds: readonly Id[]): FilmView[] {
  const wanted = new Set(folderIds);
  return films.filter((film) => wanted.has(film.folderId));
}

/** How many lines of dialogue a film has across all of its subtitle tracks. */
export function linesOf(film: FilmView): number {
  return film.tracks.reduce((total, track) => total + track.cueCount, 0);
}

/** The file name at the end of a path, whichever way the platform writes them. */
export function fileNameOf(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}
