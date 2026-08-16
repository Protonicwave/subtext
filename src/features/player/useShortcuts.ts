import { useEffect, useRef } from 'react';
import { SKIP_MS } from './intervals';
import type { Stepping } from './useStepping';
import { STEP_MS, type Sync } from './useSync';
import type { TrackChoice } from './useTrack';
import type { Transport } from './usePlayback';

/**
 * The keys a film is watched with.
 *
 * Bound on the window rather than on the player, because the point of them is
 * that they work without having clicked anything first. What they do not do is
 * take keys away from whatever has focus: the sliders in the control bar handle
 * their own arrows, and a key pressed while one of them is focused belongs to
 * it.
 *
 * Escape is not here. The shell already uses it to go back, and the platform
 * uses it to leave full screen.
 */

/** Everything a key can do, which is everything the control bar can do. */
export interface Actions {
  transport: Transport;
  stepping: Stepping;
  /** Moving the subtitles against the film, which is done by ear. */
  sync: Sync;
  /** Working out where they should have been by listening to the film. */
  align: () => void;
  /** Which subtitle is being read, and whether there is a choice to make. */
  choice: TrackChoice;
  toggleFullscreen: () => void;
  toggleSync: () => void;
  toggleTracks: () => void;
  /** Something happened, so the controls should be on screen to show what. */
  wake: () => void;
}

export function useShortcuts(actions: Actions) {
  // The listener is bound once for the life of the player and reads what the
  // actions are at the moment a key is pressed. Binding it to them instead
  // would mean unbinding and rebinding the whole set every time any one of them
  // was made afresh, which is every render for anything derived from the film.
  const latest = useRef(actions);
  useEffect(() => {
    latest.current = actions;
  });

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
      if (isTyping(event.target)) return;
      if (isCovered()) return;

      const {
        transport,
        stepping,
        sync,
        align,
        choice,
        toggleFullscreen,
        toggleSync,
        toggleTracks,
        wake,
      } = latest.current;

      // What the arrows mean, which is whether the film has any dialogue for
      // them to land on. Somebody pressing the left arrow missed a line, and
      // the line is where they meant to go; a film with no subtitle has
      // nothing to land on and moves by the interval instead.
      const byLine = stepping.available;

      switch (event.key) {
        case ' ':
        case 'k':
          transport.toggle();
          break;
        case 'ArrowLeft':
          if (byLine) stepping.back();
          else transport.skipBy(-SKIP_MS);
          break;
        case 'ArrowRight':
          if (byLine) stepping.on();
          else transport.skipBy(SKIP_MS);
          break;
        case 'm':
          transport.toggleMute();
          break;
        case 'f':
          toggleFullscreen();
          break;
        /*
         * The brackets, because they read as moving something one way or the
         * other and neither is spoken for. A film with no subtitle has nothing
         * to move, and the key then does nothing rather than reporting a value
         * for a track that is not there.
         */
        case '[':
          if (!sync.available) return;
          sync.nudge(-STEP_MS);
          break;
        case ']':
          if (!sync.available) return;
          sync.nudge(STEP_MS);
          break;
        case 's':
          if (!sync.available) return;
          toggleSync();
          break;
        /*
         * A for align, beside the keys that do the same thing by hand. A film
         * with no subtitle has nothing to measure, so the key does nothing
         * rather than starting a reading that would be turned away.
         */
        case 'a':
          if (!sync.available) return;
          align();
          break;
        /*
         * C for captions, which is what every other player calls this key. A
         * film with no subtitle has nothing to choose between, and the key then
         * does nothing rather than opening an empty menu.
         */
        case 'c':
          if (!choice.available) return;
          toggleTracks();
          break;
        default:
          return;
      }

      // Something happened, so the controls should be on screen to show what.
      wake();
      // Space scrolls whatever is behind, and the arrows move the focused
      // control. Neither is what was meant by pressing them over a film.
      event.preventDefault();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);
}

/**
 * Whether something is over the film, which the keys then belong to.
 *
 * The import sheet and the shortcut card are both modal dialogs, and both can
 * be opened over a film that is playing. A field in one of them takes the keys
 * that are typed into it, but the buttons beside it do not: space on one of
 * those would press the button and start the film behind it at once.
 */
function isCovered(): boolean {
  return document.querySelector('dialog[open]') !== null;
}

/** Whether the key belongs to a control that reads keys itself. */
function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;

  return (
    target.isContentEditable ||
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement
  );
}
