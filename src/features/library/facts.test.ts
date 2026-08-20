import { describe, expect, it } from 'vitest';
import type { FilmView, MediaView } from '@/shared/ipc/bindings';
import { fileFactsOf } from './facts';

const film = {
  id: 1,
  folderId: 1,
  path: '/films/Crime/Heat.1995.mkv',
  shelf: { name: 'Crime', path: '/films/Crime' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: 170 * 60_000,
  posterPath: null,
  coverSource: 'none',
  accent: null,
  missing: false,
  details: null,
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

const details = {
  container: 'MKV',
  sizeBytes: 15_800_000_000,
  averageBitrate: 12_400_000,
  video: { codec: 'H.264', width: 1920, height: 1080, bitDepth: 8, frameRate: 23.976 },
  audio: [],
} satisfies MediaView;

/** The rows as a table, which is how a test asks what the sheet would show. */
function factsOf(changes: Partial<FilmView>): Record<string, string> {
  const rows = fileFactsOf({ ...film, ...changes }, new Date(Date.UTC(2026, 7, 15)));
  return Object.fromEntries(rows.map((row) => [row.label, row.value]));
}

describe('what the sheet says a file is', () => {
  it('reports what the container stated', () => {
    expect(factsOf({ details })).toEqual({
      Container: 'MKV',
      Video: 'H.264',
      Resolution: '1920 × 1080',
      'Bit depth': '8-bit',
      'Frame rate': '23.976 fps',
      'Average bitrate': '12.4 Mb/s',
      Size: '15.8 GB',
      Added: '12 Aug',
      Path: '/films/Crime/Heat.1995.mkv',
    });
  });

  /*
   * An MP4 is not parsed, so it carries what the filesystem and the name can
   * supply and nothing else. Showing less is correct; a row of noughts where
   * the picture should be described would not be.
   */
  it('leaves out the picture entirely for a film whose container is not read', () => {
    const facts = factsOf({ details: { ...details, video: null } });

    expect(facts).not.toHaveProperty('Video');
    expect(facts).not.toHaveProperty('Resolution');
    expect(facts).not.toHaveProperty('Bit depth');
    expect(facts).not.toHaveProperty('Frame rate');
    expect(facts.Container).toBe('MKV');
    expect(facts.Size).toBe('15.8 GB');
  });

  it('leaves out one fact the file did not state without losing the rest', () => {
    const facts = factsOf({
      details: { ...details, video: { ...details.video, bitDepth: null, frameRate: null } },
    });

    expect(facts).not.toHaveProperty('Bit depth');
    expect(facts).not.toHaveProperty('Frame rate');
    expect(facts.Resolution).toBe('1920 × 1080');
  });

  /*
   * A film recorded before this build described such things. It still has a
   * path and a date, which are the two facts that never came from the file.
   */
  it('still says where a film is and when it arrived when nothing was read', () => {
    expect(factsOf({})).toEqual({ Added: '12 Aug', Path: '/films/Crime/Heat.1995.mkv' });
  });

  it('gives a bitrate no film has as nothing rather than as nought', () => {
    expect(factsOf({ details: { ...details, averageBitrate: null } })).not.toHaveProperty(
      'Average bitrate',
    );
  });

  /*
   * 23.976 and 24 are what the framerate conversions in the timing panel exist
   * to tell apart, so the sheet has to be able to tell them apart as well.
   */
  it('keeps the places that distinguish one framerate from its neighbour', () => {
    const rate = (frameRate: number) =>
      factsOf({ details: { ...details, video: { ...details.video, frameRate } } })['Frame rate'];

    expect(rate(23.976)).toBe('23.976 fps');
    expect(rate(24)).toBe('24 fps');
    expect(rate(29.97)).toBe('29.970 fps');
  });

  it('says a small film in the units it is small in', () => {
    const facts = factsOf({
      details: { ...details, sizeBytes: 240_000_000, averageBitrate: 800_000 },
    });

    expect(facts.Size).toBe('240 MB');
    expect(facts['Average bitrate']).toBe('800 kb/s');
  });

  it('marks the path as one, so it can be cut from the left', () => {
    const path = fileFactsOf(film).find((row) => row.label === 'Path');
    expect(path?.path).toBe(true);
  });
});
