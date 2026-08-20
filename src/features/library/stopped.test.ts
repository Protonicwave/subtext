import { describe, expect, it } from 'vitest';
import type { FilmView, TrackView } from '@/shared/ipc/bindings';
import { ANY_LANGUAGE } from '@/shared/media/languages';
import { leftOf, stoppedOn, watchedWithOf } from './stopped';

const track = {
  id: 3,
  path: '/films/Heat.1995.en.srt',
  language: 'en',
  forced: false,
  hearingImpaired: false,
  origin: 'sidecar',
  streamNumber: 0,
  form: 'text',
  matchKind: 'exact',
  correction: { offsetMs: 0, rate: 1 },
  cueCount: 1_402,
} satisfies TrackView;

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  shelf: { name: 'films', path: '/films' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: 170 * 60_000,
  posterPath: null,
  coverPath: null,
  coverSource: 'none',
  accent: null,
  missing: false,
  details: null,
  tracks: [track],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

/** A film stopped a given number of minutes into a film of a given length. */
function stoppedAt(minutes: number, of: number | null = 170): FilmView {
  return {
    ...film,
    position: {
      positionMs: minutes * 60_000,
      durationMs: of === null ? null : of * 60_000,
      finished: false,
      updatedAt: 0,
      progress: of === null ? null : minutes / of,
    },
  };
}

describe('how much is left, against how long', () => {
  it('names the running time the figure is a share of', () => {
    expect(leftOf(stoppedAt(122))).toBe('48 min left of 2 hr 50 min');
  });

  /*
   * Neither of these is a measurement, and "Nearly finished of 2 hr 50" is not
   * a sentence.
   */
  it('leaves the running time off a statement that does not take it', () => {
    expect(leftOf(stoppedAt(170))).toBe('Nearly finished');
    expect(leftOf(film)).toBe('Carry on');
  });

  it('says what it can when the running time is not known', () => {
    expect(leftOf({ ...stoppedAt(10, null), durationMs: null })).toBe('Carry on');
  });
});

describe('when a film was stopped', () => {
  const today = new Date(2026, 7, 20, 21, 30);

  it('calls today and yesterday by their names', () => {
    expect(stoppedOn(new Date(2026, 7, 20, 9, 0).getTime(), today)).toBe('Stopped today');
    expect(stoppedOn(new Date(2026, 7, 19, 23, 59).getTime(), today)).toBe('Stopped yesterday');
  });

  it('gives the date itself once it is further back than that', () => {
    expect(stoppedOn(new Date(2026, 7, 14).getTime(), today)).toBe('Stopped on 14 Aug');
    expect(stoppedOn(new Date(2025, 10, 2).getTime(), today)).toBe('Stopped on 2 Nov 2025');
  });

  /* A machine whose clock has been put back, which is not worth a sentence of
     its own. */
  it('reads a moment in the future as today', () => {
    expect(stoppedOn(new Date(2026, 7, 22).getTime(), today)).toBe('Stopped today');
  });

  it('says nothing precise about a moment it cannot read', () => {
    expect(stoppedOn(Number.NaN, today)).toBe('Stopped earlier');
  });
});

describe('what a film is being watched with', () => {
  it('names the language of the track the player would draw', () => {
    expect(watchedWithOf(film, ANY_LANGUAGE)).toBe('English subtitles');
  });

  it('says so when subtitles have been turned off', () => {
    expect(watchedWithOf({ ...film, subtitlesOff: true }, ANY_LANGUAGE)).toBe('Subtitles off');
  });

  it('says nothing at all when there is nothing readable to watch it with', () => {
    expect(watchedWithOf({ ...film, tracks: [] }, ANY_LANGUAGE)).toBeNull();
    expect(
      watchedWithOf({ ...film, tracks: [{ ...track, form: 'pictures' }] }, ANY_LANGUAGE),
    ).toBeNull();
  });

  it('still says there are subtitles when the track does not say what they are', () => {
    expect(watchedWithOf({ ...film, tracks: [{ ...track, language: null }] }, ANY_LANGUAGE)).toBe(
      'With subtitles',
    );
  });

  /* The chosen track wins, which is what makes this the same answer the player
     arrives at rather than a second guess at it. */
  it('follows a choice that was made for the film', () => {
    const french = { ...track, id: 4, language: 'fr' } satisfies TrackView;
    const chosen = { ...film, tracks: [track, french], chosenTrackId: 4 };

    expect(watchedWithOf(chosen, ANY_LANGUAGE)).toBe('French subtitles');
  });
});
