import type { AudioView, FilmView, TrackView } from '@/shared/ipc/bindings';
import { activeTrackOf, trackProblemOf } from '@/features/player/tracks';
import { languageNamed } from '@/shared/media/languages';

/**
 * Everything a film carries, sound and subtitles alike, as the rows of one
 * table.
 *
 * One table rather than two lists because they are the same kind of thing: a
 * stream inside a file, in a language, written in some format, one of which is
 * the one that will be used. Reading them against each other is what tells
 * somebody that the film is Japanese with English subtitles beside it, and two
 * separate blocks make that comparison a scroll rather than a glance.
 *
 * A cell the film view does not carry is empty rather than filled with a
 * placeholder, which is the same rule the plate of facts above the table
 * follows: what was not stated is not shown.
 */

export interface TrackRow {
  /** Stable across redraws, since a film can carry two identical tracks. */
  key: string;
  kind: 'Sound' | 'Subtitle';
  /** The language as a person would say it, where the track claims one. */
  language: string | null;
  codec: string | null;
  /** What this track is, beyond its language and its format. */
  what: string | null;
  /** Whether it is the one that will be used, or why it cannot be. */
  state: TrackState | null;
}

export interface TrackState {
  name: string;
  /** Something to know rather than something wrong, drawn in the amber. */
  warn: boolean;
}

/**
 * The rows for one film, sound first.
 *
 * Sound first because a film always has some and its subtitles are the thing
 * being chosen between: the answer to "what is this file" reads better ending
 * on the part somebody acts on.
 */
export function trackRowsOf(film: FilmView, preferred: string): TrackRow[] {
  const active = activeTrackOf(film, preferred);
  const audio = film.details?.audio ?? [];

  return [
    ...audio.map((sound, at) => soundRow(sound, at)),
    ...film.tracks.map((track) => subtitleRow(track, track.id === active?.id)),
  ];
}

/**
 * One sound track.
 *
 * Marked as the default rather than as chosen, because nobody chose it: the
 * file says which track it suggests and the player takes that one. Subtext
 * offers no way to pick another, so calling the rest available would promise
 * something the player does not do, and they are left unmarked.
 */
function soundRow(audio: AudioView, at: number): TrackRow {
  return {
    key: `sound-${String(at)}`,
    kind: 'Sound',
    language: named(audio.language),
    codec: audio.codec,
    what: audio.layout,
    state: audio.default ? { name: 'Default', warn: false } : null,
  };
}

/**
 * One subtitle track.
 *
 * The codec column is empty for every one of them, because the film view
 * carries what a track is written as only as far as whether it can be read at
 * all. Naming the format from the file's extension would be a guess dressed as
 * a fact, and the state column already says the part that matters, which is
 * whether there is dialogue in it to draw.
 */
function subtitleRow(track: TrackView, chosen: boolean): TrackRow {
  const problem = trackProblemOf(track);

  return {
    key: `subtitle-${String(track.id)}`,
    kind: 'Subtitle',
    language: named(track.language),
    codec: null,
    what: whatOf(track),
    state: problem === null ? chosenState(chosen) : { name: problem, warn: true },
  };
}

function chosenState(chosen: boolean): TrackState | null {
  return chosen ? { name: 'Chosen', warn: false } : null;
}

/**
 * What a subtitle track is: where it came from, how much of it there is, and
 * what it is for where it is for something in particular.
 *
 * A track of pictures has no line count, since nothing was read out of it, and
 * saying "nought lines" would read as a track that failed rather than one that
 * was never going to be read.
 */
function whatOf(track: TrackView): string {
  const parts = [track.origin === 'stream' ? 'Inside the film' : 'Beside the film'];

  if (track.cueCount > 0) parts.push(`${track.cueCount.toLocaleString('en-GB')} lines`);
  if (track.forced) parts.push('Forced');
  if (track.hearingImpaired) parts.push('For the hard of hearing');

  return parts.join(' · ');
}

/** A language code as a person would say it, or nothing where there is none. */
function named(code: string | null): string | null {
  if (code === null) return null;

  const name = languageNamed(code);
  return name === '' ? null : name;
}
