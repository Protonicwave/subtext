import { useCallback, useEffect, useState } from 'react';
import type { FilmView } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';
import { useLibrary } from '@/features/library/useLibrary';

/**
 * Putting a subtitle back in step with its film, by ear.
 *
 * Nudging is provisional. The value on screen is applied through the timeline
 * the player already builds, which costs one pass over the cues and nothing per
 * frame, and it is written to the database only once somebody has stopped
 * moving it. Writing every intermediate step would mean a write and a re-read
 * of the whole track for each press of a key.
 *
 * What is written is the value the back end kept, read back rather than
 * assumed, and the cues are then read again with it applied. Until they arrive
 * the lines in hand carry the old offset and the provisional shift covers the
 * difference, so there is no moment where the correction is applied twice or
 * not at all.
 */

/** How far one press of a nudge key moves the subtitles. */
export const STEP_MS = 50;

/**
 * How long after the last nudge the value is kept.
 *
 * Long enough to press the key several times in a row and have that count as
 * one adjustment, short enough that letting go and closing the window a moment
 * later still keeps what was arrived at.
 */
const SETTLE_MS = 1_200;

/**
 * The conversions between the framerates films are actually released at.
 *
 * A fixed list rather than a slider. A rate is a ratio between two framerates,
 * there are only a few pairs in circulation, and a number arrived at by
 * dragging is a number nobody can arrive at twice. Anyone who does need
 * something else can type it.
 */
export const RATES: readonly { value: number; name: string }[] = [
  { value: 1, name: 'As the file is' },
  { value: 25 / 23.976, name: '23.976 to 25 fps' },
  { value: 23.976 / 25, name: '25 to 23.976 fps' },
  { value: 24 / 23.976, name: '23.976 to 24 fps' },
  { value: 23.976 / 24, name: '24 to 23.976 fps' },
  { value: 30 / 29.97, name: '29.97 to 30 fps' },
];

export interface Sync {
  /** Whether the film has a subtitle to put in step at all. */
  available: boolean;
  /** The offset in force, whether or not it has been written down yet. */
  offsetMs: number;
  rate: number;
  /** The value has just been moved and is waiting to be kept. */
  pending: boolean;
  /** Moves the subtitles, later for a positive amount and earlier for a negative one. */
  nudge: (byMs: number) => void;
  setRate: (rate: number) => void;
  /** Back to the file exactly as it was written. */
  reset: () => void;
}

export function useSync(film: FilmView): Sync {
  const track = film.tracks[0];
  const trackId = track?.id ?? null;
  const kept = track?.correction.offsetMs ?? 0;
  const rate = track?.correction.rate ?? 1;
  const replace = useLibrary((library) => library.replace);

  // Null while the value on screen is the value in the database, which is all
  // of the time except the second or so after somebody has moved it.
  const [draft, setDraft] = useState<number | null>(null);
  const [pending, setPending] = useState(false);
  const offsetMs = draft ?? kept;

  const commit = useCallback(
    (offset: number, stretch: number) => {
      if (trackId === null) return;
      setPending(false);
      void ipc
        .setTrackCorrection(trackId, { offsetMs: offset, rate: stretch })
        .then((updated) => {
          replace(updated);
          setDraft(null);
        })
        .catch(() => {
          // The value could not be kept. It stays on screen, so the film reads
          // right for this sitting, which is more use than putting the
          // subtitles back out of step to make a point about the database.
        });
    },
    [trackId, replace],
  );

  const nudge = useCallback(
    (byMs: number) => {
      if (trackId === null) return;
      setDraft((current) => (current ?? kept) + byMs);
      setPending(true);
    },
    [trackId, kept],
  );

  // Each nudge restarts this, so a run of presses is one write at the end of it
  // rather than one write each.
  useEffect(() => {
    if (!pending) return;

    const settling = setTimeout(() => {
      commit(offsetMs, rate);
    }, SETTLE_MS);
    return () => {
      clearTimeout(settling);
    };
  }, [pending, offsetMs, rate, commit]);

  return {
    available: trackId !== null,
    offsetMs,
    rate,
    pending,
    nudge,
    // A rate is chosen rather than felt for, so there is nothing to wait for.
    setRate: useCallback(
      (chosen: number) => {
        commit(offsetMs, chosen);
      },
      [commit, offsetMs],
    ),
    reset: useCallback(() => {
      setDraft(0);
      commit(0, 1);
    }, [commit]),
  };
}

/** The offset as the readout says it, always signed so nought reads as nought. */
export function offsetLabel(offsetMs: number): string {
  if (offsetMs === 0) return 'as written';
  const sign = offsetMs > 0 ? '+' : '−';
  return `${sign}${(Math.abs(offsetMs) / 1000).toFixed(2)}s`;
}
