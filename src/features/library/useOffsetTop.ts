import { useEffect, useState, type RefObject } from 'react';

/**
 * How far an element sits below the top of the thing that scrolls it.
 *
 * The wall is virtualised against the page's own scrolling, and the billboard
 * above it is not part of the wall. Without this the virtualiser would take
 * every scroll position to be a position within the wall and draw rows that are
 * a billboard's height too far down.
 *
 * Measured again when the scroller changes size, since the billboard is a
 * fraction of the window's height and a resized window moves the wall.
 */
export function useOffsetTop(scroller: RefObject<HTMLElement | null>, node: HTMLElement | null) {
  const [top, setTop] = useState(0);

  useEffect(() => {
    const box = scroller.current;
    if (!node || !box) return;

    const measure = () => {
      // Through the scroll position rather than through offsetTop, which is
      // measured from whichever ancestor happens to be positioned.
      const above = node.getBoundingClientRect().top - box.getBoundingClientRect().top;
      setTop(Math.max(0, Math.round(above + box.scrollTop)));
    };
    measure();

    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', measure);
      return () => {
        window.removeEventListener('resize', measure);
      };
    }

    const observer = new ResizeObserver(measure);
    observer.observe(box);
    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, [scroller, node]);

  return top;
}
