import { useEffect, useState } from 'react';

/**
 * Whether an element is close enough to the view to be worth drawing.
 *
 * A library of a hundred folders is a hundred rails, and a rail that nobody can
 * see costs the same as one they can. Each row keeps its height whether it has
 * drawn its tiles or not, so the page is as long as it will ever be from the
 * first paint and nothing moves under the scrollbar as rows fill in.
 *
 * The margin is generous on purpose: a row should have drawn itself well before
 * it reaches the bottom of the window, so that scrolling never waits for one.
 *
 * Everything is nearby where there is no observer to say otherwise, which is
 * what happens under test and in anything old enough not to have one. Drawing
 * every row is slower and not wrong.
 */
export function useNearby(margin = 900): [(node: HTMLElement | null) => void, boolean] {
  const [node, setNode] = useState<HTMLElement | null>(null);
  const [nearby, setNearby] = useState(typeof IntersectionObserver === 'undefined');

  useEffect(() => {
    if (!node || typeof IntersectionObserver === 'undefined') return;

    const observer = new IntersectionObserver(
      (entries) => {
        setNearby(entries.some((entry) => entry.isIntersecting));
      },
      { rootMargin: `${String(margin)}px 0px` },
    );

    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, [node, margin]);

  return [setNode, nearby];
}
