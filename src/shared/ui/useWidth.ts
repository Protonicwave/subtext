import { useEffect, useState } from 'react';

/**
 * How wide an element is, kept up to date as the window changes.
 *
 * The poster grid cannot lay itself out without this: how many tiles fit across
 * decides how many rows there are, and the virtualiser needs the number of rows
 * before anything is drawn.
 *
 * It hands back a ref to attach rather than taking one, because the element it
 * measures may not be in the document on the first render, and a ref object
 * gives nothing to notice when it arrives.
 *
 * Zero until the element has been measured, which callers read as not knowing
 * yet rather than as an element of no width.
 *
 * The element itself comes back alongside, for a caller that has something else
 * to measure about the same element and would otherwise need a second ref on
 * it.
 */
export function useWidth(): [(node: HTMLElement | null) => void, number, HTMLElement | null] {
  const [node, setNode] = useState<HTMLElement | null>(null);
  const [width, setWidth] = useState(0);

  useEffect(() => {
    if (!node) return;

    const measure = () => {
      setWidth(node.clientWidth);
    };
    measure();

    // Watching the element rather than the window catches a panel opening
    // beside it, which a resize event says nothing about.
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', measure);
      return () => {
        window.removeEventListener('resize', measure);
      };
    }

    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, [node]);

  return [setNode, width, node];
}
