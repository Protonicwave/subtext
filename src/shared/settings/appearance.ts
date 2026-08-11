import { useEffect } from 'react';
import { useSetting } from './useSettings';

/**
 * Puts the appearance settings where the stylesheet can see them.
 *
 * Two custom properties and one attribute on the root element, which is all
 * three of these are: the ambient wash and the grain are drawn from tokens, and
 * the motion preference is a set of durations the tokens already redefine for
 * the system's own setting.
 *
 * Written on the root rather than passed down, because what they change is the
 * window itself and everything in it, and threading three values through every
 * component to reach two pseudo-elements would be absurd.
 */
export function useAppearance(): void {
  const glow = useSetting('glow');
  const grain = useSetting('grain');
  const motion = useSetting('motion');

  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty('--ambient', String(glow));
    root.style.setProperty('--grain-opacity', String(grain));
  }, [glow, grain]);

  useEffect(() => {
    // Following the system is the absence of an opinion, so nothing is said
    // and the media query in the tokens is left to answer.
    if (motion === 'system') delete document.documentElement.dataset.motion;
    else document.documentElement.dataset.motion = motion;
  }, [motion]);
}
