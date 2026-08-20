import { describe, expect, it } from 'vitest';
import type { FilmView, MediaView, TrackView } from '@/shared/ipc/bindings';
import { ANY_LANGUAGE } from '@/shared/media/languages';
import { trackRowsOf } from './trackRows';

const track = {
  id: 3,
  path: '/films/Heat.1995.en.srt',
  origin: 'sidecar',
  streamNumber: 0,
  form: 'text',
  language: 'en',
  forced: false,
  hearingImpaired: false,
  matchKind: 'exact',
  cueCount: 1_402,
  correction: { offsetMs: 0, rate: 1 },
} satisfies TrackView;

const details = {
  container: 'MKV',
  sizeBytes: 15_800_000_000,
  averageBitrate: 12_400_000,
  video: { codec: 'H.264', width: 1920, height: 1080, bitDepth: 8, frameRate: 23.976 },
  audio: [{ codec: 'DTS-HD MA', layout: '5.1', language: 'en', default: true }],
} satisfies MediaView;

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
  details,
  tracks: [track],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

/** The rows for a film, with nobody having asked for a language in particular. */
function rowsOf(changes: Partial<FilmView> = {}) {
  return trackRowsOf({ ...film, ...changes }, ANY_LANGUAGE);
}

describe('a film s tracks as rows', () => {
  it('puts the sound first and the subtitles after it', () => {
    expect(rowsOf().map((row) => row.kind)).toEqual(['Sound', 'Subtitle']);
  });

  it('says what a sound track is, and which one will be heard', () => {
    const [sound] = rowsOf();

    expect(sound).toMatchObject({
      kind: 'Sound',
      language: 'English',
      codec: 'DTS-HD MA',
      what: '5.1',
      state: { name: 'Default', warn: false },
    });
  });

  /*
   * Nothing in Subtext picks a sound track: the file says which it suggests and
   * the player takes that one. Marking the others as available would offer
   * something no control does.
   */
  it('leaves a sound track the film does not suggest unmarked', () => {
    const rows = rowsOf({
      details: {
        ...details,
        audio: [
          { codec: 'DTS-HD MA', layout: '5.1', language: 'en', default: true },
          { codec: 'AC-3', layout: '2.0', language: 'fr', default: false },
        ],
      },
    });

    expect(rows[1]?.state).toBeNull();
  });

  it('says where a subtitle came from and how much of it there is', () => {
    const rows = rowsOf();

    expect(rows[1]).toMatchObject({
      kind: 'Subtitle',
      language: 'English',
      what: 'Beside the film · 1,402 lines',
      state: { name: 'Chosen', warn: false },
    });
  });

  it('says what a track inside the film is for, where it is for something', () => {
    const rows = rowsOf({
      tracks: [
        { ...track, id: 4, origin: 'stream', streamNumber: 2, forced: true, hearingImpaired: true },
      ],
    });

    expect(rows[1]?.what).toBe('Inside the film · 1,402 lines · Forced · For the hard of hearing');
  });

  /*
   * Nought lines would read as a track that failed. A track of pictures was
   * never going to be read, and the state column is where that is said.
   */
  it('names a track of pictures as one it cannot read, and counts no lines', () => {
    const rows = rowsOf({
      tracks: [{ ...track, form: 'pictures', origin: 'stream', cueCount: 0 }],
    });

    expect(rows[1]).toMatchObject({
      what: 'Inside the film',
      state: { name: 'Pictures, not text', warn: true },
    });
  });

  it('marks the one the film would be watched with, and only that one', () => {
    const rows = rowsOf({
      tracks: [track, { ...track, id: 4, language: 'fr' }],
      chosenTrackId: 4,
    });

    expect(rows[1]?.state).toBeNull();
    expect(rows[2]?.state).toMatchObject({ name: 'Chosen' });
  });

  it('marks none where the subtitles have been turned off', () => {
    expect(rowsOf({ subtitlesOff: true })[1]?.state).toBeNull();
  });

  /*
   * The film view says whether a subtitle can be read and not what it is
   * written as. Naming the format from the file's extension would be a guess
   * dressed as a fact, and an empty cell is what the rest of the page does with
   * something the file did not state.
   */
  it('leaves a subtitle s format empty rather than guessing at it', () => {
    expect(rowsOf()[1]?.codec).toBeNull();
  });

  it('leaves a language empty where the track claims none', () => {
    const rows = rowsOf({
      details: {
        ...details,
        audio: [{ codec: 'AAC', layout: null, language: null, default: true }],
      },
      tracks: [{ ...track, language: null }],
    });

    expect(rows[0]?.language).toBeNull();
    expect(rows[1]?.language).toBeNull();
  });

  it('has nothing to say about a film whose file was never read', () => {
    expect(rowsOf({ details: null, tracks: [] })).toEqual([]);
  });

  /*
   * Two tracks of one film can say the same thing about themselves, so the key
   * comes from what tells them apart rather than from what they say.
   */
  it('keys every row apart, even where two of them read alike', () => {
    const rows = rowsOf({
      details: {
        ...details,
        audio: [
          { codec: 'AAC', layout: '2.0', language: 'en', default: true },
          { codec: 'AAC', layout: '2.0', language: 'en', default: false },
        ],
      },
      tracks: [track, { ...track, id: 4 }],
    });

    expect(new Set(rows.map((row) => row.key)).size).toBe(rows.length);
  });
});
