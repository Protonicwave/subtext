import { useEffect, useState } from 'react';
import type { CueView, FilmView, Id } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';

/**
 * The dialogue of the film being played.
 *
 * Read once, whole, when the player opens. A film of five thousand cues is a
 * few hundred kilobytes and the alternative is asking across the boundary while
 * the film is running, which is the one thing the index exists to avoid.
 *
 * A film with no subtitles is not a failure and produces no message: it plays,
 * and there is nothing to draw over it.
 */

/** The lines of a film, and what had already been done to their timings. */
export interface Dialogue {
  readonly cues: readonly CueView[];
  /**
   * The offset these cues came back with, already applied to them.
   *
   * The correction is applied in Rust as the cues are read, so what arrives
   * here are playback timings. Which offset they went through still matters to
   * one caller: the sync control, whose provisional value is the difference
   * between what is being tried and what these lines already have. Without it,
   * the moment a value is committed and before the fresh cues arrive, the
   * player would apply the whole offset a second time.
   */
  readonly offsetMs: number;
}

/** One value for every film that has nothing, so that none of them redraw. */
const NOTHING: Dialogue = { cues: [], offsetMs: 0 };

interface Read extends Dialogue {
  /** Which track the lines belong to, and how it was read. */
  track: Id | null;
  rate: number;
}

export function useCues(film: FilmView | undefined): Dialogue {
  const chosen = film?.tracks[0];
  const track = chosen?.id ?? null;
  // Read again when the correction changes, since the cues carry it. The offset
  // and the rate are named separately rather than as the object they came in,
  // which is a fresh one on every read of the library.
  const offsetMs = chosen?.correction.offsetMs ?? 0;
  const rate = chosen?.correction.rate ?? 1;

  // Which track the cues belong to is kept with them, so that a film opened
  // while another was still being read cannot end up with the other one's
  // dialogue on screen.
  const [read, setRead] = useState<Read>({ ...NOTHING, track: null, rate: 1 });

  useEffect(() => {
    if (track === null) return;

    let wanted = true;
    void ipc
      .trackCues(track)
      .then((cues) => {
        if (wanted) setRead({ track, offsetMs, rate, cues });
      })
      .catch(() => {
        // The subtitles could not be read. The film still plays, which is the
        // better half of what was asked for, and a message over the picture
        // would be worse than the silence.
        if (wanted) setRead({ ...NOTHING, track, rate });
      });

    return () => {
      wanted = false;
    };
  }, [track, offsetMs, rate]);

  // The lines in hand are kept while a correction is being read again, rather
  // than the film going quiet for as long as that takes. They are a little out
  // of date and the player knows by how much, which is what the offset above
  // is for.
  return read.track === track ? read : NOTHING;
}
