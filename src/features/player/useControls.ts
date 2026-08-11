import { useCallback, useEffect, useRef, useState } from 'react';
import { PLAYBACK } from './defaults';

/**
 * When the controls are on screen.
 *
 * Driven by what the pointer is doing rather than by a timer alone. Moving the
 * pointer anywhere over the film brings them back; leaving it still while the
 * film plays takes them away again. A paused film keeps them, because a paused
 * film is one somebody is doing something with, and controls that vanish from
 * under a resting pointer are controls that have to be hunted for.
 *
 * Being paused and being held under the pointer are read at the moment they are
 * asked about rather than turning the timer on and off, which is what keeps this
 * to one piece of state and no synchronising. The timer only ever says that
 * nothing has happened lately; whether that means the controls go is a separate
 * question and a simpler one.
 *
 * Waking is called on every pointer move, so it does as little as it can: the
 * state is already awake almost every time, React drops a set that changes
 * nothing, and all that is left is a timer being restarted.
 */
export interface Controls {
  visible: boolean;
  /** Something happened. Show the controls and start the clock again. */
  wake: () => void;
  /** The pointer is over the controls themselves, so they stay put. */
  hold: (holding: boolean) => void;
}

export function useControls(playing: boolean): Controls {
  const [awake, setAwake] = useState(true);
  const [held, setHeld] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const wake = useCallback(() => {
    setAwake(true);
    if (timer.current !== null) clearTimeout(timer.current);

    timer.current = setTimeout(() => {
      setAwake(false);
    }, PLAYBACK.hideAfterMs);
  }, []);

  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current);
    },
    [],
  );

  return { visible: awake || !playing || held, wake, hold: setHeld };
}
