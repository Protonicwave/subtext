import type { GroundsView, LandingView } from '@/shared/ipc/bindings';

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

/**
 * What the film said about an answer, as an outcome reports it.
 *
 * The same idea as the landing figures above: an alignment that was written
 * carries the stretches it was borne out by, and a test about the wording of
 * something else should not have to invent a plausible set of them.
 */
export function grounds(score: number, agreed = 18, across = 20): GroundsView {
  return { agreed, across, spreadMs: 40, score };
}
