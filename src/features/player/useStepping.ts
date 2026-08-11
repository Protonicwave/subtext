import { useMemo } from 'react';
import { type Timeline, nextFrom, previousFrom } from '@/shared/media/cues';
import type { Transport } from './usePlayback';

/**
 * Moving through a film by what is said in it.
 *
 * Ten seconds is an arbitrary distance. It lands mid-word as often as not, and
 * going back because you missed a line means going back to the start of that
 * line, not to a moment ten seconds ago that may be in the middle of the line
 * before it. This is the whole of what the index buys the player: the film has
 * places in it, and they are the lines.
 *
 * Where the film is comes from the element rather than from the last
 * `timeupdate`, which can be a quarter of a second stale. A quarter of a second
 * is enough to step to the wrong line at the edges of one.
 */
export interface Stepping {
  /** Whether there is any dialogue to step through. */
  available: boolean;
  /** Back to the start of the line being spoken, or to the one before it. */
  back: () => void;
  /** On to the next thing said. */
  on: () => void;
}

export function useStepping(timeline: Timeline, transport: Transport): Stepping {
  return useMemo(() => {
    // Nothing found means nothing to do: past the last line there is no next
    // one, and before the first there is no line to go back to.
    const goTo = (found: number) => {
      const cue = timeline.cues[found];
      if (cue !== undefined) transport.seekTo(cue.startMs);
    };

    return {
      available: timeline.cues.length > 0,
      back: () => {
        goTo(previousFrom(timeline, transport.positionNow()));
      },
      on: () => {
        goTo(nextFrom(timeline, transport.positionNow()));
      },
    };
  }, [timeline, transport]);
}
