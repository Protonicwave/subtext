import { useEffect } from 'react';
import type { AccentView } from '@/shared/ipc/bindings';

/**
 * Gives the window the colours of the film that is open.
 *
 * The tokens are written so that everything downstream reads the accent from
 * the root element, which is what makes this two lines rather than a context
 * threaded through every component. The ambient wash behind the window is on a
 * long transition, so moving between films reads as a change in the light.
 *
 * Put back on the way out, so the library is its own colour again rather than
 * the colour of whatever was last watched.
 */
export function useFilmAccent(accent: AccentView | null) {
  const primary = accent?.primary;
  const pair = accent?.pair;

  useEffect(() => {
    if (primary === undefined || pair === undefined) return;

    const root = document.documentElement;
    root.style.setProperty('--colour-accent', primary);
    root.style.setProperty('--colour-accent-pair', pair);

    return () => {
      root.style.removeProperty('--colour-accent');
      root.style.removeProperty('--colour-accent-pair');
    };
  }, [primary, pair]);
}
