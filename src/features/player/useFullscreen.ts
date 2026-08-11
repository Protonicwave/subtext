import { type RefObject, useCallback, useEffect, useState } from 'react';

/**
 * Filling the screen with the film.
 *
 * The platform's own full screen rather than a window of ours drawn over
 * everything, so that the title bar, the taskbar and the second monitor all
 * behave the way they do for every other player. Escape leaves it, which the
 * platform handles and we do not have to.
 */
export function useFullscreen(target: RefObject<HTMLElement | null>): [boolean, () => void] {
  const [full, setFull] = useState(false);

  useEffect(() => {
    const onChanged = () => {
      setFull(document.fullscreenElement !== null);
    };
    document.addEventListener('fullscreenchange', onChanged);
    return () => {
      document.removeEventListener('fullscreenchange', onChanged);
    };
  }, []);

  const toggle = useCallback(() => {
    const element = target.current;
    // Under test, and on a platform without it, there is nothing to fill.
    if (element === null || typeof element.requestFullscreen !== 'function') return;

    const asked =
      document.fullscreenElement === null ? element.requestFullscreen() : document.exitFullscreen();

    void asked.catch(() => {
      // Refused, which leaves the film where it was. The button will simply
      // have done nothing, which is the whole of what went wrong.
    });
  }, [target]);

  return [full, toggle];
}
