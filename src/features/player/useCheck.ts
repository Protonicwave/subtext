import { useCallback, useState } from 'react';
import type { AlignmentView, CueView } from '@/shared/ipc/bindings';
import { busiestMomentOf } from './check';
import type { Alignment } from './useAlignment';
import type { Transport } from './usePlayback';

/**
 * Watching a measurement land, rather than taking it on trust.
 *
 * An alignment ends with a sentence full of numbers, and the numbers are the
 * honest ones: how many lines land on the talking now against how many did
 * before, and how much of the film agreed. None of them is what somebody
 * actually wants, which is to see a line arrive as somebody speaks. So the last
 * thing a measurement does is offer to show them: the film goes to its busiest
 * few seconds of dialogue and plays, and the question underneath is whether to
 * keep it.
 *
 * Putting it back is the undo that was already there. This adds a place to press
 * it from and a reason to have an opinion, and no second way of undoing
 * anything.
 *
 * Only offered for a measurement that was written, since there is nothing to
 * check about a refusal: the file is exactly as it was and watching it would
 * show what it always showed.
 */

export interface Check {
  /** There is a measurement in force that was worth checking. */
  offered: boolean;
  /** The film is at the moment in question and running. */
  watching: boolean;
  /** Go there and play it. */
  start: () => void;
  /** The lines land. The measurement stands and the question goes away. */
  keep: () => void;
  /** They do not. Back to whatever was in force before the measurement. */
  putBack: () => void;
}

export function useCheck(
  alignment: Alignment,
  cues: readonly CueView[],
  transport: Transport,
): Check {
  // The one outcome there is anything to check. A refusal changed nothing, and
  // a measurement that has already been put back is not in force to be judged.
  const written =
    alignment.state.phase === 'outcome' &&
    alignment.state.outcome.outcome === 'aligned' &&
    !alignment.state.undone
      ? alignment.state.outcome
      : null;

  const [watching, setWatching] = useState(false);
  const [answered, setAnswered] = useState(false);

  // A fresh measurement is a fresh question, whatever was said about the last
  // one. Noticed as it is rendered rather than in an effect, so that the offer
  // is never drawn for a moment as though it had already been answered.
  const [about, setAbout] = useState<AlignmentView | null>(written);
  if (about !== written) {
    setAbout(written);
    setWatching(false);
    setAnswered(false);
  }

  const { undo } = alignment;

  return {
    // A track whose lines could not be read has nowhere to send anybody, and an
    // offer that would decline is worse than no offer.
    offered: written !== null && !answered && !watching && cues.length > 0,
    watching: written !== null && watching,
    start: useCallback(() => {
      const at = busiestMomentOf(cues);
      if (at === null) return;

      transport.seekTo(at);
      // Playing rather than toggling: this is somebody asking to watch a stretch
      // of film, and on a film they had paused to read the sentence, a toggle
      // would leave them looking at a still.
      transport.play();
      setWatching(true);
    }, [cues, transport]),
    keep: useCallback(() => {
      setWatching(false);
      setAnswered(true);
    }, []),
    putBack: useCallback(() => {
      undo();
      setWatching(false);
    }, [undo]),
  };
}
