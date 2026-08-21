import { useState } from 'react';
import type { FilmView, Id } from '@/shared/ipc/bindings';
import { ipc, reasonFor } from '@/shared/ipc/client';
import { useLibrary } from './useLibrary';

/**
 * Settling which picture a film is shown with.
 *
 * Three ways in and one path through: the picker, a drop, and giving the film
 * back to the scan. All of them end by putting the row the back end returned
 * into the library, which is what redraws the tile and, because the poster is
 * filed under the cover it was drawn from, what makes the poster loop notice
 * that a new one is wanted.
 *
 * Nothing here writes anywhere near the film. The image stays where it was and
 * the row records where to find it.
 */

interface Cover {
  /** True while the picker is open or a command is in flight. */
  busy: boolean;
  /** What went wrong, in the sentence the back end wrote for it. */
  problem: string | null;
  /** Opens the picker, and settles on whatever comes back. */
  choose: () => void;
  /** Settles on a picture whose path is already known, as a drop's is. */
  settle: (path: string) => void;
  /** Gives the film back to the scan. */
  clear: () => void;
  dismiss: () => void;
}

export function useCover(film: FilmView): Cover {
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  const run = (work: () => Promise<FilmView | null>) => {
    setBusy(true);
    setProblem(null);
    void work()
      .then((updated) => {
        if (updated !== null) useLibrary.getState().replace(updated);
      })
      .catch((failure: unknown) => {
        setProblem(reasonFor(failure));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  return {
    busy,
    problem,
    choose: () => {
      run(async () => {
        const path = await ipc.chooseImage();
        // Nothing was picked, which is not a failure and not a change either.
        return path === null ? null : ipc.chooseCover(film.id, path);
      });
    },
    settle: (path) => {
      run(() => ipc.chooseCover(film.id, path));
    },
    clear: () => {
      run(() => ipc.clearCover(film.id));
    },
    dismiss: () => {
      setProblem(null);
    },
  };
}

/**
 * Settling a cover from outside a component, which is what a drop is.
 *
 * A drop is reported to the window rather than to whatever was under the
 * pointer, so the film it landed on is worked out afterwards and there is no
 * component in hand to ask. The library is updated the same way, so a tile
 * redraws whether the picture arrived through the picker or through a drop.
 */
export async function settleCover(filmId: Id, path: string): Promise<void> {
  const updated = await ipc.chooseCover(filmId, path);
  useLibrary.getState().replace(updated);
}
