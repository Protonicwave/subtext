import { useEffect } from 'react';
import type { FilmView, Id, PosterWanted } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';
import { streamOf } from '@/shared/media/source';
import { captureFrom, posterFrom } from './capture';
import { keyFor, useFrames } from './frames';
import { useLibrary } from './useLibrary';

/**
 * Drawing the pictures the library shows itself with.
 *
 * Background work in the ordinary sense: one film at a time, yielding to the
 * window between each, and never anything a person is waiting for. A folder
 * that has just been added shows its tiles straight away and they take on their
 * pictures over the following minute.
 *
 * Everything here comes from the same loop because it all wants the same
 * things: a worker, a decoder for the films that need one, and to be the only
 * one running. The posters are cached to disk and the back end says which are
 * wanted and where each of them comes from; the frames for the continue
 * watching row are held in memory for as long as the window is open.
 */

/**
 * Films a capture has already failed on.
 *
 * Kept for the life of the window rather than in a store, so that a file the
 * webview will not decode is tried once and not once per redraw. A restart
 * tries again, which is the right frequency for something that installing a
 * codec would fix.
 */
const refused = new Set<Id>();

export function useCapture(ready: boolean) {
  useEffect(() => {
    // No worker means no capture: under test, and anywhere else without one,
    // every film draws its generated tile and nothing goes wrong.
    if (!ready || typeof Worker === 'undefined') return;

    const abort = new AbortController();
    const worker = new Worker(new URL('./poster.worker.ts', import.meta.url), { type: 'module' });

    void run(worker, abort.signal);

    return () => {
      abort.abort();
      worker.terminate();
    };
  }, [ready]);
}

/** Takes what is missing, then waits for the library to change and looks again. */
async function run(worker: Worker, signal: AbortSignal) {
  while (!signal.aborted) {
    await posters(worker, signal);
    await resumeFrames(worker, signal);
    await changed(signal);
  }
}

/** The films with no poster cached, or whose poster is no longer the right one. */
async function posters(worker: Worker, signal: AbortSignal) {
  let wanted;
  try {
    wanted = await ipc.postersWanted();
  } catch {
    // The library could not be asked. Whatever is wrong with it is already
    // being said somewhere a person can see it, and a missing poster is not
    // the thing to say it a second time about.
    return;
  }

  for (const film of wanted) {
    if (signal.aborted) return;
    if (refused.has(film.id)) continue;
    await idle();

    try {
      const drawn = await drawnFrom(film, worker);
      const updated = await ipc.savePoster(
        film.id,
        [...new Uint8Array(drawn.image)],
        drawn.accent,
        drawn.durationMs,
      );
      useLibrary.getState().replace(updated);
    } catch {
      // A film that cannot be opened, decoded or written is a film with a
      // composed tile, which is a complete answer rather than a failure.
      refused.add(film.id);
    }
  }
}

/**
 * The image a film's poster is made from.
 *
 * Which of the two it is was decided during the scan and is on the film's row;
 * all that is left here is to fetch an image or to open the film. The second is
 * far more expensive, which is why it is only reached when there was no image.
 */
async function drawnFrom(film: PosterWanted, worker: Worker) {
  if (!film.cover) return captureFrom(streamOf(film.path), worker);

  const bytes = await ipc.coverImage(film.id);
  return posterFrom(new Uint8Array(bytes), worker);
}

/** A frame from where each partly watched film was left. */
async function resumeFrames(worker: Worker, signal: AbortSignal) {
  for (const film of useLibrary.getState().resumable) {
    if (signal.aborted) return;
    if (film.missing || film.position === null || refused.has(film.id)) continue;

    const key = keyFor(film.id, film.position.positionMs);
    if (useFrames.getState().frames[film.id]?.key === key) continue;

    await idle();
    try {
      const { image } = await captureFrom(streamOf(film.path), worker, {
        seconds: film.position.positionMs / 1000,
      });
      useFrames.getState().keep(film.id, key, image);
    } catch {
      refused.add(film.id);
    }
  }
}

/** Whether a film has a moment worth showing rather than its poster. */
export function resumesAt(film: FilmView): number | null {
  return film.position === null || film.missing ? null : film.position.positionMs;
}

/** Waits for the window to have nothing better to do. */
function idle(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof requestIdleCallback !== 'function') {
      setTimeout(resolve, 0);
      return;
    }
    // The deadline matters. A window nobody is touching is idle for ever, and
    // the posters should still arrive.
    requestIdleCallback(
      () => {
        resolve();
      },
      { timeout: 2_000 },
    );
  });
}

/** Waits for the library to change, or for the window to be closing. */
function changed(signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    const stop = () => {
      unsubscribe();
      signal.removeEventListener('abort', stop);
      resolve();
    };
    const unsubscribe = useLibrary.subscribe(stop);
    signal.addEventListener('abort', stop, { once: true });
  });
}
