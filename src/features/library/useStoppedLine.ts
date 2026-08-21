import { useEffect, useState } from 'react';
import type { CueView, FilmView } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';
import { NONE, nearestFrom, timelineOf } from '@/shared/media/cues';
import { useSetting } from '@/shared/settings/useSettings';
import { activeTrackOf } from '@/features/player/tracks';

/**
 * The line that was on screen where a film was stopped.
 *
 * This is the part of the masthead worth the work. Every player can say that
 * forty eight minutes are left; only one that has already parsed and indexed
 * the dialogue can answer "where was I" in the film's own words. The cues are
 * on a row in the database before the film is ever opened, so the answer costs
 * one read and no decoding.
 *
 * The whole track is read rather than the one line, because that is the command
 * that exists and a track is a few hundred kilobytes. Asking for a single cue
 * would be a command, a query and a migration's worth of index for a saving
 * nobody would notice on a read that happens once.
 */

/**
 * How far either side of the saved position still counts as the moment somebody
 * stopped at.
 *
 * People pause between lines as often as during one, so the nearest line within
 * a quarter of a minute is still what was being said there. Beyond that the
 * film is quiet at that point and the honest answer is to say nothing.
 */
const AROUND_MS = 15_000;

/**
 * How far in counts as having started the film at all.
 *
 * A film opened and closed again has a position of a second or two and nothing
 * has been said in it yet. Reading its dialogue to print the opening titles
 * would be work in aid of nothing.
 */
const STARTED_MS = 1_000;

/** What was read, and which film and moment it was read for. */
interface Read {
  key: string;
  line: CueView | null;
}

export function useStoppedLine(film: FilmView): CueView | null {
  const preferred = useSetting('subtitleLanguage');

  // The track the player would draw, so that a film watched with subtitles
  // turned off prints no line, and one watched in French prints the French.
  const trackId = activeTrackOf(film, preferred)?.id ?? null;
  const positionMs = film.position?.positionMs ?? 0;
  const wanted = trackId !== null && positionMs >= STARTED_MS;

  // Which film and moment the line belongs to is kept with it, so that a
  // library that changed under a read in flight cannot end up printing one
  // film's dialogue against another film's title.
  const key = `${String(trackId)}:${String(positionMs)}`;
  const [read, setRead] = useState<Read>({ key: '', line: null });

  useEffect(() => {
    if (!wanted) return;

    let alive = true;
    /*
     * A frame later, so the shelves are drawn before anything is asked for.
     * Nothing on this screen depends on the answer, and the library is what
     * somebody opened the window to see.
     */
    const frame = requestAnimationFrame(() => {
      void ipc
        .trackCues(trackId)
        .then((cues) => {
          if (!alive) return;

          const timeline = timelineOf(cues);
          const at = nearestFrom(timeline, positionMs, AROUND_MS);
          setRead({ key, line: at === NONE ? null : (timeline.cues[at] ?? null) });
        })
        .catch(() => {
          // The dialogue could not be read, which is a line the masthead does
          // without. Everything else it says is still true.
          if (alive) setRead({ key, line: null });
        });
    });

    return () => {
      alive = false;
      cancelAnimationFrame(frame);
    };
  }, [wanted, trackId, positionMs, key]);

  // Nothing at all while a different film's line is still in hand, rather than
  // the last one lingering under a title it does not belong to.
  return read.key === key ? read.line : null;
}
