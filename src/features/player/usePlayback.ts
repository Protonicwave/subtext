import { type RefObject, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { whyUnplayable } from './unplayable';

/**
 * The video element, as the rest of the player sees it.
 *
 * The element is the truth: it holds the position, it decides whether it is
 * playing, and it is the only thing that knows how long the film is. So nothing
 * here keeps a second copy that could disagree with it. This listens to what it
 * says and hands back the last thing it said, and every action is a call on the
 * element that comes back round as an event.
 *
 * That is also why the position here is updated from `timeupdate`, four times a
 * second, rather than from the frame loop. The clock in the control bar shows
 * whole seconds and the bar moves a pixel every few seconds, so a redraw per
 * frame would be sixty times the work for a picture nobody could tell apart.
 */

export interface Playback {
  /** The file has been opened far enough to know what it is. */
  ready: boolean;
  playing: boolean;
  /** Waiting for bytes, which is what the spinner is for. */
  waiting: boolean;
  positionMs: number;
  durationMs: number | null;
  volume: number;
  muted: boolean;
  /** Why the film will not play, once it is clear that it will not. */
  problem: string | null;
}

export interface Transport {
  toggle: () => void;
  seekTo: (ms: number) => void;
  skipBy: (ms: number) => void;
  setVolume: (volume: number) => void;
  toggleMute: () => void;
}

const START: Playback = {
  ready: false,
  playing: false,
  waiting: true,
  positionMs: 0,
  durationMs: null,
  volume: 1,
  muted: false,
  problem: null,
};

/**
 * The element is made here rather than passed in, so that the one piece of the
 * player that writes to it is also the one that owns it. What comes back is the
 * ref to attach, what it is doing, and what may be done to it.
 *
 * Nothing resets when the film changes, because the film cannot change: the
 * player is mounted afresh for each one, which is the only way to be sure no
 * part of the last film's state is left over.
 *
 * @param path The film's path on disk, which is what names the likely reason
 *   for a file that will not play.
 * @param startAtMs Where to begin, which is where the film was left.
 */
export function usePlayback(
  path: string,
  startAtMs: number,
): [RefObject<HTMLVideoElement | null>, Playback, Transport] {
  const video = useRef<HTMLVideoElement>(null);
  const [state, setState] = useState<Playback>(START);

  useEffect(() => {
    const element = video.current;
    if (element === null) return;

    const patch = (change: Partial<Playback>) => {
      setState((playback) => ({ ...playback, ...change }));
    };

    // A film with no running time reports NaN, and a position past the end
    // happens when a file is replaced by a shorter cut of the same film.
    const duration = () => (Number.isFinite(element.duration) ? element.duration * 1000 : null);

    let started = false;
    const listeners: [keyof HTMLMediaElementEventMap, () => void][] = [
      [
        'loadedmetadata',
        () => {
          const length = duration();
          // Where the film was left. Done here rather than after it can play,
          // so that the first bytes fetched are the ones that will be shown.
          if (startAtMs > 0) {
            element.currentTime =
              Math.max(0, Math.min(startAtMs, (length ?? startAtMs) - 1_000)) / 1000;
          }
          patch({ ready: true, durationMs: length, positionMs: element.currentTime * 1000 });

          // Opening a film means watching it, and the click that opened it is
          // the gesture the platform wants before sound is allowed. Asked for
          // here rather than once it can play, because a file loaded no
          // further than its metadata may never reach that on its own.
          if (started) return;
          started = true;
          void element.play().catch(() => {
            // Refused, which leaves the film on its first frame with the
            // controls showing. Nothing to say that the play button does not.
          });
        },
      ],
      [
        'durationchange',
        () => {
          patch({ durationMs: duration() });
        },
      ],
      [
        'timeupdate',
        () => {
          patch({ positionMs: element.currentTime * 1000 });
        },
      ],
      // A seek is the one moment the readout has to move before the next
      // timeupdate, or the controls lag a quarter of a second behind the film.
      [
        'seeked',
        () => {
          patch({ positionMs: element.currentTime * 1000 });
        },
      ],
      [
        'play',
        () => {
          patch({ playing: true });
        },
      ],
      [
        'pause',
        () => {
          patch({ playing: false });
        },
      ],
      [
        'ended',
        () => {
          patch({ playing: false });
        },
      ],
      [
        'waiting',
        () => {
          patch({ waiting: true });
        },
      ],
      [
        'playing',
        () => {
          patch({ waiting: false, playing: true });
        },
      ],
      [
        'volumechange',
        () => {
          patch({ volume: element.volume, muted: element.muted });
        },
      ],
      [
        'canplay',
        () => {
          patch({ waiting: false });
        },
      ],
      [
        'error',
        () => {
          patch({
            problem: whyUnplayable(element.error?.code, path),
            waiting: false,
            playing: false,
          });
        },
      ],
    ];

    for (const [event, handler] of listeners) element.addEventListener(event, handler);
    return () => {
      for (const [event, handler] of listeners) element.removeEventListener(event, handler);
    };
  }, [path, startAtMs]);

  const seekTo = useCallback((ms: number) => {
    const element = video.current;
    if (element === null) return;

    const length = Number.isFinite(element.duration) ? element.duration * 1000 : null;
    const at = Math.max(0, length === null ? ms : Math.min(ms, length));
    element.currentTime = at / 1000;
  }, []);

  const toggle = useCallback(() => {
    const element = video.current;
    if (element === null) return;

    if (element.paused) {
      void element.play().catch(() => {
        // Reported by the error event if it is the file, and not worth saying
        // twice if it is not.
      });
    } else {
      element.pause();
    }
  }, []);

  const skipBy = useCallback(
    (ms: number) => {
      const element = video.current;
      if (element !== null) seekTo(element.currentTime * 1000 + ms);
    },
    [seekTo],
  );

  const setVolume = useCallback((volume: number) => {
    const element = video.current;
    if (element === null) return;

    element.volume = Math.max(0, Math.min(1, volume));
    // Dragging the slider up from nothing means wanting to hear it.
    element.muted = volume === 0;
  }, []);

  const toggleMute = useCallback(() => {
    const element = video.current;
    if (element === null) return;

    // Coming back from a slider dragged all the way down, where unmuting on its
    // own would be silence with the mute button off.
    if (element.muted && element.volume === 0) element.volume = 1;
    element.muted = !element.muted;
  }, []);

  // Held together as one value, and a stable one: the keyboard shortcuts bind
  // to it, and a new object on every timeupdate would rebind them four times a
  // second for the length of the film.
  const transport = useMemo(
    () => ({ toggle, seekTo, skipBy, setVolume, toggleMute }),
    [toggle, seekTo, skipBy, setVolume, toggleMute],
  );

  return [video, state, transport];
}
