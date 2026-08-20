import type { FilmView } from '@/shared/ipc/bindings';
import { activeTrackOf } from '@/features/player/tracks';
import { languageNamed } from '@/shared/media/languages';
import { dateOf } from './dates';
import { FINISHING_MS, leftMsOf, remainingOf } from './remaining';
import { runtimeOf } from './runtime';

/**
 * What the library screen says about where somebody stopped.
 *
 * Three statements, each of them a fact the application already holds and none
 * of them a recommendation. The screen a personal library opens on should
 * answer "where was I" and stop there, since there is nothing here to sell and
 * nothing to rank.
 */

/**
 * How much is left, against how long the whole film is.
 *
 * The running time is only worth naming beside a figure it is a share of.
 * "Nearly finished" and "Carry on" are descriptions of where somebody got to
 * rather than measurements of it, and neither takes an "of" after it.
 */
export function leftOf(film: FilmView): string {
  const said = remainingOf(film);
  const left = leftMsOf(film);
  if (left === null || left < FINISHING_MS) return said;

  const whole = runtimeOf(film.position?.durationMs ?? film.durationMs);
  return whole === null ? said : `${said} of ${whole}`;
}

/**
 * When a film was stopped, said the way somebody would say it.
 *
 * Today and yesterday by name, because that is what a person calls them, and
 * the date itself beyond that rather than a count of days nobody would work
 * back from. A moment in the future is a machine whose clock has moved and
 * reads as today, which is near enough true and better than "in three days".
 *
 * The moment to measure against is passed in rather than read here, so that
 * what this returns depends only on what it was given.
 */
export function stoppedOn(millis: number, today = new Date()): string {
  const date = dateOf(millis, today);
  if (date === '') return 'Stopped earlier';

  const days = daysBetween(millis, today);
  if (days <= 0) return 'Stopped today';
  if (days === 1) return 'Stopped yesterday';
  return `Stopped on ${date}`;
}

/**
 * Which subtitles the film is being watched with, where it is being watched
 * with any.
 *
 * The track the player would draw and no other, worked out by the same rule the
 * player uses, so that the screen cannot name one subtitle and the film open on
 * another. A film with nothing readable says nothing at all: an empty statement
 * about subtitles is worse than the silence.
 */
export function watchedWithOf(film: FilmView, preferred: string): string | null {
  if (film.subtitlesOff) return 'Subtitles off';

  const track = activeTrackOf(film, preferred);
  if (track === null) return null;

  // A track can carry a language the platform has no name for, and one can
  // carry no language at all. Both come to the same thing here, which is that
  // there are subtitles and nothing to call them.
  const named = track.language === null ? '' : languageNamed(track.language);
  return named === '' ? 'With subtitles' : `${named} subtitles`;
}

/** How many whole days apart two moments are, counted by the date each falls on. */
function daysBetween(millis: number, today: Date): number {
  const then = new Date(millis);
  const from = Date.UTC(then.getFullYear(), then.getMonth(), then.getDate());
  const to = Date.UTC(today.getFullYear(), today.getMonth(), today.getDate());

  return Math.round((to - from) / 86_400_000);
}
