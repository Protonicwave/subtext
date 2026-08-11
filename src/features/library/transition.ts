import type { Id } from '@/shared/ipc/bindings';

/**
 * The name a film's frame goes by while it is moving.
 *
 * A poster in the grid and the frame the player opens into are the same element
 * as far as the animation is concerned: they share this identifier, and what
 * happens between one and the other is worked out from where each of them is.
 * Both ends have to agree on the name, so neither writes it.
 */
export function frameId(id: Id): string {
  return `film-frame-${String(id)}`;
}

/** Whether the machine has been told to keep movement to a minimum. */
export function stillnessWanted(): boolean {
  return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}
