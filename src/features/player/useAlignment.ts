import { useCallback, useEffect, useRef, useState } from 'react';
import type { AlignStageView, AlignmentView, TrackView } from '@/shared/ipc/bindings';
import { events } from '@/shared/ipc/bindings';
import { ipc, reasonFor } from '@/shared/ipc/client';
import { useIpcEvent } from '@/shared/ipc/useIpcEvent';
import { type Sync, asWritten } from './useSync';

/**
 * Asking the back end to work out where a subtitle should have been.
 *
 * The measurement, the threshold and every way it can end belong to Rust. What
 * is here is the asking, the waiting and the putting back, which is four states
 * and no more: nothing happening, a question about replacing what somebody
 * arrived at by ear, the reading with a way to stop it, and what came of it.
 * Dismissing the last of those is a return to the first, so it needs no state
 * of its own.
 *
 * It lives beside the film rather than inside the timing panel, because the
 * panel can be closed with the S key while the reading is still going on and
 * closing it should not lose the answer.
 */

export type AlignState =
  | { phase: 'idle' }
  /** A correction is in force, and going ahead would replace it. */
  | { phase: 'confirming' }
  | { phase: 'running'; stage: AlignStageView; fraction: number }
  /** It ended, one way or another. Only `aligned` changed anything. */
  | { phase: 'outcome'; outcome: AlignmentView; undone: boolean };

export interface Alignment {
  /** Whether there is a subtitle to measure at all. */
  available: boolean;
  state: AlignState;
  /**
   * Measures the track against the film, asking first where that would throw
   * away a value somebody arrived at themselves.
   */
  start: () => void;
  /** Yes, replace what is in force. */
  confirm: () => void;
  /** Stop reading. The track is left exactly as it was found. */
  cancel: () => void;
  /** Put back the correction the alignment replaced. */
  undo: () => void;
  dismiss: () => void;
}

export function useAlignment(track: TrackView | null, sync: Sync): Alignment {
  const trackId = track?.id ?? null;
  const [state, setState] = useState<AlignState>({ phase: 'idle' });

  // Read at the moment they are wanted rather than depended on, since both are
  // made afresh on every render of the film and neither is worth tearing an
  // event subscription down for.
  const latest = useRef({ trackId, sync });
  useEffect(() => {
    latest.current = { trackId, sync };
  });

  // Only about the subtitle on screen, and only while one is being read. A
  // reading somebody has switched away from carries on and is written where it
  // belongs, but it has nothing to say here any more.
  useIpcEvent(events.alignProgressed, (progress) => {
    if (progress.trackId !== trackId || state.phase !== 'running') return;
    setState({ phase: 'running', stage: progress.stage, fraction: progress.fraction });
  });

  // Switching subtitle takes the answer with it, since it was about a track
  // nobody is reading now. Adjusted as it is noticed rather than in an effect,
  // so that nothing is drawn about the old track first. Nothing is stopped: the
  // back end finishes what it was asked for and writes it against the track it
  // was asked about, which is what somebody who asked and then went to make tea
  // would expect.
  const [about, setAbout] = useState(trackId);
  if (about !== trackId) {
    setAbout(trackId);
    setState({ phase: 'idle' });
  }

  const measure = useCallback(() => {
    const asked = latest.current.trackId;
    if (asked === null) return;

    setState({ phase: 'running', stage: 'reading', fraction: 0 });

    void ipc
      .alignTrack(asked)
      .then(async (outcome) => {
        // Somebody has switched to a different subtitle while this was
        // reading. It was written for the track it was asked about, which is
        // the right thing to have done, but it is no longer what is on screen
        // and saying so over a different track would be a report about nothing.
        if (latest.current.trackId !== asked) return;

        // The correction was written to the row before this resolved, so the
        // copy the player is reading is now a version behind. Writing the same
        // value again is how it is read back, and it costs one statement
        // against a value that is already there.
        if (outcome.outcome === 'aligned') await latest.current.sync.apply(outcome.correction);
        setState({ phase: 'outcome', outcome, undone: false });
      })
      .catch((failure: unknown) => {
        // Only the library itself failing gets here. Everything the alignment
        // can decline is an outcome above, so this is rare enough to say in the
        // same place as the rest rather than somewhere of its own.
        setState({
          phase: 'outcome',
          outcome: { outcome: 'unreadable', message: reasonFor(failure) },
          undone: false,
        });
      });
  }, []);

  const start = useCallback(() => {
    if (latest.current.trackId === null) return;

    // Nothing records where a correction came from, and nothing needs to, so
    // anything other than the file as written is treated as somebody's own
    // work and asked about. That includes a value this put there a moment ago,
    // which is the right way round: it is still a value on screen that is about
    // to be replaced by a different one.
    const { offsetMs, rate } = latest.current.sync;
    if (!asWritten({ offsetMs, rate })) {
      setState({ phase: 'confirming' });
      return;
    }
    measure();
  }, [measure]);

  return {
    available: trackId !== null,
    state,
    start,
    confirm: measure,
    cancel: useCallback(() => {
      // The answer still comes back through the call that is already waiting,
      // as a stop rather than as a correction, so there is nothing to set here.
      void ipc.cancelAlignment().catch(() => {
        // Nothing was running, or it had already finished. Either way the
        // outcome about to arrive says what happened.
      });
    }, []),
    undo: useCallback(() => {
      if (state.phase !== 'outcome' || state.outcome.outcome !== 'aligned') return;
      void latest.current.sync.apply(state.outcome.previous);
      setState({ ...state, undone: true });
    }, [state]),
    dismiss: useCallback(() => {
      setState({ phase: 'idle' });
    }, []),
  };
}
