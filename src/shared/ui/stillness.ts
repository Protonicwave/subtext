import { useSetting } from '@/shared/settings/useSettings';

/**
 * Whether movement is to be kept to a minimum.
 *
 * The tokens answer the system's own setting for everything that is a duration,
 * which is most of it. This is for the few places that are not a duration: a
 * preview that plays by itself, a poster that starts moving under the pointer.
 * Those cannot be shortened to a millisecond, they can only be left undone.
 */

/** What the machine has been told, ignoring what the application has been. */
export function stillnessWanted(): boolean {
  return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/**
 * What to do here, taking both the machine and the application into account.
 *
 * Either of them is enough, which is how the tokens read the pair as well: the
 * preference can only ask for less movement, never for more than the system has
 * already said it wants.
 */
export function useStillness(): boolean {
  const motion = useSetting('motion');
  return motion === 'reduced' || stillnessWanted();
}
