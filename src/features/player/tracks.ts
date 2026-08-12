import type { FilmView, TrackView } from '@/shared/ipc/bindings';
import { fileNameOf } from '@/features/library/useLibrary';
import { ANY_LANGUAGE, languageNamed } from '@/shared/media/languages';

/**
 * Which of a film's subtitles is read, and how each of them is described.
 *
 * A film can be paired with several: the same dialogue in three languages, a
 * forced track for the signs, a second one written for viewers who cannot hear
 * the audio. Taking whichever came back first is a coin toss, and a forced
 * track is the one it lands on often enough to be worth a rule.
 *
 * Nothing here reads state or writes any. It is given the film and what the
 * settings say, and it says which track that comes to.
 */

/**
 * The track a film is watched with, or nothing where there is none to read.
 *
 * A choice that was made wins outright. Where none has been, the rule below
 * decides, and it decides again every time it is asked: a film given a better
 * subtitle by a later scan opens on that one without anybody having to notice.
 */
export function activeTrackOf(film: FilmView, preferred: string): TrackView | null {
  if (film.subtitlesOff) return null;

  // A choice naming a track this film no longer has is not an error worth
  // saying anything about: a subtitle attached to a different film, or one
  // deleted between scans, both come to the same thing, which is a film that
  // needs choosing for again.
  const chosen = film.tracks.find((track) => track.id === film.chosenTrackId);
  return chosen ?? defaultTrackOf(film.tracks, preferred);
}

/**
 * The track to open a film on when nobody has chosen one.
 *
 * In order: the preferred language, then a track that is not forced, then one
 * that is not written for viewers who cannot hear the audio, then whichever
 * came first. Forced tracks caption signs and the odd line in another language
 * in a film that is otherwise understood, so opening on one means a film that
 * looks as though it has almost no subtitles at all.
 */
export function defaultTrackOf(tracks: readonly TrackView[], preferred: string): TrackView | null {
  let best: TrackView | null = null;
  let score = -1;

  for (const track of tracks) {
    const rank = rankOf(track, preferred);
    // Strictly better, so a tie is settled by the order the tracks came in,
    // which is the order the library lists them.
    if (rank > score) {
      best = track;
      score = rank;
    }
  }

  return best;
}

/**
 * How well a track answers, as one number.
 *
 * The three questions are asked in order of how much they matter, and each is
 * worth more than everything under it put together, so a preferred language
 * beats any combination of the rest.
 */
function rankOf(track: TrackView, preferred: string): number {
  const language = preferred !== ANY_LANGUAGE && track.language === preferred ? 4 : 0;
  return language + (track.forced ? 0 : 2) + (track.hearingImpaired ? 0 : 1);
}

/** What a track is called: its language, or the file it came from. */
export function trackNameOf(track: TrackView): string {
  if (track.language === null) return fileNameOf(track.path);
  return languageNamed(track.language);
}

/** What else is worth saying about a track, where there is anything. */
export function trackNoteOf(track: TrackView): string | null {
  const notes: string[] = [];
  if (track.forced) notes.push('Forced');
  if (track.hearingImpaired) notes.push('For the hard of hearing');
  if (notes.length === 0) return null;
  return notes.join(' · ');
}
