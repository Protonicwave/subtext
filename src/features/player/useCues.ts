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

/** One array for every film that has nothing, so that none of them redraw. */
const NOTHING: readonly CueView[] = [];

export function useCues(film: FilmView | undefined): readonly CueView[] {
  const track = film?.tracks[0]?.id ?? null;
  // Which track the cues belong to is kept with them, so that a film opened
  // while another was still being read cannot end up with the other one's
  // dialogue on screen.
  const [read, setRead] = useState<{ track: Id | null; cues: readonly CueView[] }>({
    track: null,
    cues: NOTHING,
  });

  useEffect(() => {
    if (track === null) return;

    let wanted = true;
    void ipc
      .trackCues(track)
      .then((cues) => {
        if (wanted) setRead({ track, cues });
      })
      .catch(() => {
        // The subtitles could not be read. The film still plays, which is the
        // better half of what was asked for, and a message over the picture
        // would be worse than the silence.
        if (wanted) setRead({ track, cues: NOTHING });
      });

    return () => {
      wanted = false;
    };
  }, [track]);

  return read.track === track ? read.cues : NOTHING;
}
