import { create } from 'zustand';
import type { Id } from '@/shared/ipc/bindings';

/**
 * Which film has its page open.
 *
 * A store rather than state on the library screen, because a film is opened
 * from four places that do not share a parent: a tile, a row of the list, the
 * billboard, and the palette from anywhere at all. The film itself is not held
 * here, only which one it is, so that a sheet left open while a scan changes
 * the row underneath it draws what the library now says rather than a copy
 * taken when it was opened.
 */
interface SheetState {
  filmId: Id | null;
  open: (filmId: Id) => void;
  close: () => void;
}

export const useSheet = create<SheetState>((set) => ({
  filmId: null,
  open: (filmId) => {
    set({ filmId });
  },
  close: () => {
    set({ filmId: null });
  },
}));
