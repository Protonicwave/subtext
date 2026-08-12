import { describe, expect, it } from 'vitest';
import type { FilmView, Id, TrackView } from '@/shared/ipc/bindings';
import { ANY_LANGUAGE } from '@/shared/media/languages';
import { activeTrackOf, defaultTrackOf, trackNameOf, trackNoteOf } from './tracks';

function track(id: Id, changes: Partial<TrackView> = {}): TrackView {
  return {
    id,
    path: `/films/Heat.${String(id)}.srt`,
    language: null,
    forced: false,
    hearingImpaired: false,
    matchKind: 'exact',
    cueCount: 900,
    correction: { offsetMs: 0, rate: 1 },
    ...changes,
  };
}

function film(tracks: TrackView[], changes: Partial<FilmView> = {}): FilmView {
  return {
    id: 7,
    folderId: 1,
    path: '/films/Heat.1995.mkv',
    title: 'Heat',
    year: 1995,
    durationMs: null,
    posterPath: null,
    accent: null,
    missing: false,
    tracks,
    chosenTrackId: null,
    subtitlesOff: false,
    position: null,
    ...changes,
  };
}

describe('picking a track for a film nobody has chosen for', () => {
  it('finds nothing in a film with no subtitles', () => {
    expect(defaultTrackOf([], 'en')).toBeNull();
  });

  it('takes the only one there is', () => {
    const only = track(1, { language: 'fr', forced: true });
    expect(defaultTrackOf([only], 'en')).toBe(only);
  });

  it('prefers the language that was asked for', () => {
    const french = track(1, { language: 'fr' });
    const english = track(2, { language: 'en' });

    expect(defaultTrackOf([french, english], 'en')).toBe(english);
    expect(defaultTrackOf([french, english], 'fr')).toBe(french);
  });

  /*
   * The case the rule exists for. A forced track is a handful of lines over
   * the signs, so a film opening on one looks as though it has no subtitles.
   */
  it('leaves a forced track for somebody who asks for it', () => {
    const forced = track(1, { language: 'en', forced: true });
    const full = track(2, { language: 'en' });

    expect(defaultTrackOf([forced, full], 'en')).toBe(full);
    expect(defaultTrackOf([forced, full], ANY_LANGUAGE)).toBe(full);
  });

  it('prefers a track that is not describing the audio as well', () => {
    const impaired = track(1, { language: 'en', hearingImpaired: true });
    const plain = track(2, { language: 'en' });

    expect(defaultTrackOf([impaired, plain], 'en')).toBe(plain);
  });

  /*
   * The preferred language is worth more than everything under it, so a
   * hearing impaired track in the language somebody reads beats a plain one in
   * a language they do not.
   */
  it('would rather have the right language than the plainest track', () => {
    const german = track(1, { language: 'de' });
    const english = track(2, { language: 'en', hearingImpaired: true });

    expect(defaultTrackOf([german, english], 'en')).toBe(english);
  });

  it('takes the first of two that are as good as each other', () => {
    const first = track(1, { language: 'en' });
    const second = track(2, { language: 'en' });

    expect(defaultTrackOf([first, second], 'en')).toBe(first);
  });

  it('falls back to the forced track when it is all there is', () => {
    const forced = track(1, { language: 'en', forced: true });
    expect(defaultTrackOf([forced], 'en')).toBe(forced);
  });
});

describe('the track a film is watched with', () => {
  it('is the one that was chosen, whatever the rule would have said', () => {
    const forced = track(1, { language: 'en', forced: true });
    const full = track(2, { language: 'en' });

    expect(activeTrackOf(film([forced, full], { chosenTrackId: 1 }), 'en')).toBe(forced);
  });

  it('is none at all where subtitles have been turned off', () => {
    const full = track(2, { language: 'en' });
    expect(activeTrackOf(film([full], { subtitlesOff: true }), 'en')).toBeNull();
  });

  /*
   * Turning them off is a different answer from never having said, which is
   * the whole reason both are stored.
   */
  it('is picked by the rule where nobody has said', () => {
    const full = track(2, { language: 'en' });
    expect(activeTrackOf(film([full]), 'en')).toBe(full);
  });

  it('falls back to the rule when the chosen track has gone', () => {
    const full = track(2, { language: 'en' });
    expect(activeTrackOf(film([full], { chosenTrackId: 404 }), 'en')).toBe(full);
  });
});

describe('describing a track', () => {
  it('names the language it is in', () => {
    expect(trackNameOf(track(1, { language: 'en' }))).toBe('English');
  });

  it('names an unlabelled track after its file', () => {
    expect(trackNameOf(track(1, { path: '/films/Heat.1995.srt' }))).toBe('Heat.1995.srt');
  });

  it('says what a track is for, where it is for anything in particular', () => {
    expect(trackNoteOf(track(1))).toBeNull();
    expect(trackNoteOf(track(1, { forced: true }))).toBe('Forced');
    expect(trackNoteOf(track(1, { hearingImpaired: true }))).toBe('For the hard of hearing');
    expect(trackNoteOf(track(1, { forced: true, hearingImpaired: true }))).toBe(
      'Forced · For the hard of hearing',
    );
  });
});
