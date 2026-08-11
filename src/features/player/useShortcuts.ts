import { useEffect } from 'react';
import { PLAYBACK } from './defaults';
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
export function useShortcuts(
  transport: Transport,
  onToggleFullscreen: () => void,
  wake: () => void,
) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
      if (isTyping(event.target)) return;

      switch (event.key) {
        case ' ':
        case 'k':
          transport.toggle();
          break;
        case 'ArrowLeft':
          transport.skipBy(-PLAYBACK.skipMs);
          break;
        case 'ArrowRight':
          transport.skipBy(PLAYBACK.skipMs);
          break;
        case 'm':
          transport.toggleMute();
          break;
        case 'f':
          onToggleFullscreen();
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
  }, [transport, onToggleFullscreen, wake]);
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
