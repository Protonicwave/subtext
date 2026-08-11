import { useEffect, useRef } from 'react';
import type { Id } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';
import { useSettings } from '@/shared/settings/useSettings';
import { useLibrary } from '@/features/library/useLibrary';
import { isFinished } from './resume';

/**
 * Writing down where a film has got to.
 *
 * On a throttle while it plays, whenever it is paused, and once on the way out.
 * The throttle is what makes a window closed without warning cost a few seconds
 * rather than the whole session: a webview being torn down is not somewhere to
 * rely on getting a last word in.
 *
 * The position comes from what the element last reported rather than from the
 * element itself, because the last write happens as the player is being taken
 * apart and by then React has already let go of it. A quarter of a second of
 * staleness is nothing; having no last write at all would mean every film
 * losing the last few seconds of where it got to.
 *
 * The running time goes with every write. A film whose poster was never
 * captured has none stored, and the library cannot draw how far through
 * something is without one.
 */

/**
 * How far in a film has to be before it is worth remembering.
 *
 * Opening a film and closing it again is not watching it, and a library full of
 * films to carry on with from the first second is a library nobody trusts.
 */
const WORTH_KEEPING = 1_000;

/**
 * How often a position is written down while a film is playing.
 *
 * Not a preference. Nobody has an opinion about it, and the only thing it
 * changes is how much of the last few seconds a window closed without warning
 * costs.
 */
const SAVE_EVERY = 5_000;

interface Watching {
  filmId: Id;
  positionMs: number;
  durationMs: number | null;
}

export function useKeepPosition(
  filmId: Id,
  positionMs: number,
  playing: boolean,
  durationMs: number | null,
) {
  // Read at the moment a save happens rather than closed over, so that a
  // position arriving four times a second does not rebuild the throttle and
  // write a row every time it does.
  const watching = useRef<Watching>({ filmId, positionMs, durationMs });
  useEffect(() => {
    watching.current = { filmId, positionMs, durationMs };
  }, [filmId, positionMs, durationMs]);

  // While it is playing. The cleanup runs when it is paused as well as when the
  // player closes, which is why pausing is not handled separately.
  useEffect(() => {
    if (!playing) return;

    const timer = setInterval(() => {
      void save(watching.current);
    }, SAVE_EVERY);

    return () => {
      clearInterval(timer);
      void save(watching.current);
    };
  }, [playing]);

  // On the way out of the player, and on the way out of the window.
  useEffect(() => {
    const onLeaving = () => {
      void save(watching.current);
    };
    window.addEventListener('pagehide', onLeaving);

    return () => {
      window.removeEventListener('pagehide', onLeaving);
      // The library screen behind this one shows what to carry on with, and it
      // is about to be looked at. Read again rather than patched, since the row
      // that has just been written decides the order of the whole list.
      void save(watching.current).then(() => useLibrary.getState().refresh());
    };
  }, []);
}

async function save({ filmId, positionMs, durationMs }: Watching): Promise<void> {
  if (positionMs < WORTH_KEEPING) return;

  try {
    await ipc.savePosition(
      filmId,
      Math.round(positionMs),
      durationMs === null ? null : Math.round(durationMs),
      isFinished(positionMs, durationMs, useSettings.getState().settings.watchedFraction),
    );
  } catch {
    // Nothing to tell somebody watching a film. The position is a convenience,
    // and the next write in a few seconds may well work.
  }
}
