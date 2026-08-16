import type { LandingView } from '@/shared/ipc/bindings';

/**
 * How many of a track's lines land on the talking, as an outcome reports it.
 *
 * Every outcome that carries a measurement carries two of these, and almost no
 * test is about the numbers in them. Written once here so that a test which is
 * about something else can say the share it wants and nothing more.
 */
export function landing(fraction: number, examined = 1_200): LandingView {
  return { fraction, medianMs: fraction > 0.5 ? 40 : 1_200, examined };
}

/** A measurement of a film that has nothing in it to measure. */
export const nothingMeasured: LandingView = { fraction: 0, medianMs: 0, examined: 0 };
