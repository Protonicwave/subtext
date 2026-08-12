import { useCallback } from 'react';
import type { FilmView, TrackView } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';
import { useLibrary } from '@/features/library/useLibrary';
import { useSetting } from '@/shared/settings/useSettings';
import { activeTrackOf } from './tracks';

/**
 * Which subtitle a film is being watched with, and changing it.
 *
 * The choice belongs to the film and is kept in the database, so this holds no
 * copy of it: what is on screen is what the library says, and choosing writes
 * the row and puts back the film that comes out of it. Guessing at the new
 * state would be quicker by about a millisecond and wrong every time a write
 * failed.
 *
 * Nothing here touches the picture. A different track is a different array of
 * cues, which the player reads as it would any other, and the film carries on
 * playing through the swap.
 */

export interface TrackChoice {
  /** Every subtitle paired with this film, in the order the library lists them. */
  tracks: readonly TrackView[];
  /** The one being read, or nothing where there is none to read. */
  active: TrackView | null;
  /** Subtitles have been turned off for this film, rather than there being none. */
  off: boolean;
  /** Whether there is anything to choose between at all. */
  available: boolean;
  /** Reads this film with that track, or with none when given nothing. */
  choose: (track: TrackView | null) => void;
}

export function useTrack(film: FilmView): TrackChoice {
  const preferred = useSetting('subtitleLanguage');
  const replace = useLibrary((library) => library.replace);
  const filmId = film.id;

  const choose = useCallback(
    (track: TrackView | null) => {
      void ipc
        .setFilmTrack(filmId, track?.id ?? null)
        .then(replace)
        .catch(() => {
          // The choice could not be kept, so the film stays on the track it
          // was already reading. Nothing is put over the picture for it: the
          // menu is still open and still says which track is in use, which is
          // the answer to what happened.
        });
    },
    [filmId, replace],
  );

  return {
    tracks: film.tracks,
    active: activeTrackOf(film, preferred),
    off: film.subtitlesOff,
    available: film.tracks.length > 0,
    choose,
  };
}
