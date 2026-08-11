import { useEffect } from 'react';
import type { AccentView } from '@/shared/ipc/bindings';
import type { Settings } from '@/shared/settings/schema';
import { useSetting } from '@/shared/settings/useSettings';
import { paletteOf } from './fallback';

/**
 * The colours one film is drawn in.
 *
 * Its own, taken from the frame captured for its poster, unless somebody would
 * rather the application kept one colour. The fixed answer names the tokens
 * rather than repeating what they hold, so there is still one place the amber
 * is written down.
 */
export function paletteFor(
  film: { title: string; accent: AccentView | null },
  accent: Settings['accent'],
): AccentView {
  return accent === 'film' ? paletteOf(film) : FIXED;
}

/** The same, for one film rather than for a list of them. */
export function useFilmPalette(film: { title: string; accent: AccentView | null }): AccentView {
  return paletteFor(film, useSetting('accent'));
}

const FIXED: AccentView = { primary: 'var(--colour-accent)', pair: 'var(--colour-accent-pair)' };

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
  // Somebody who asked for one colour is asking for the window not to move
  // with the film, so there is nothing to write on the root at all.
  const fromFilm = useSetting('accent') === 'film';
  const primary = fromFilm ? accent?.primary : undefined;
  const pair = fromFilm ? accent?.pair : undefined;

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
