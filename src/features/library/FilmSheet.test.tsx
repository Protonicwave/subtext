import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as BindingsModule from '@/shared/ipc/bindings';
import type * as ClientModule from '@/shared/ipc/client';
import type { AlignmentView, CueView, FilmView, MediaView, TrackView } from '@/shared/ipc/bindings';
import { grounds, landing } from '@/test/alignment';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    showInFolder: vi.fn(() => Promise.resolve(null)),
    // Reading a film takes a while, and this one never finishes unless a test
    // says how it ended. That is what lets the reading itself be looked at.
    alignTrack: vi.fn((): Promise<AlignmentView> => new Promise(() => undefined)),
    cancelAlignment: vi.fn(() => Promise.resolve(null)),
    setTrackCorrection: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
    trackCues: vi.fn((): Promise<CueView[]> => Promise.resolve([])),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});
// The real emitter would go looking for a window to subscribe through, and no
// test here is about what an alignment reports on the way.
vi.mock('@/shared/ipc/bindings', async () => {
  const actual = await vi.importActual<typeof BindingsModule>('@/shared/ipc/bindings');
  return {
    ...actual,
    events: {
      ...actual.events,
      alignProgressed: { listen: () => Promise.resolve(() => undefined) },
    },
  };
});
// The URL a file is served from is the shell's business, and there is no shell
// under test. The sheet only cares that it has one.
vi.mock('@/shared/media/source', () => ({ sourceOf: (path: string) => `asset://${path}` }));

const { LEAD_MS } = await import('@/features/player/check');
const { CHECK } = await import('@/features/player/outcomes');
const { FilmSheet } = await import('./FilmSheet');
const { useLibrary } = await import('./useLibrary');
const { useSheet } = await import('./useSheet');
const { useNavigation } = await import('@/app/routes');
const { useSettings } = await import('@/shared/settings/useSettings');
const { DEFAULTS } = await import('@/shared/settings/schema');

const track = {
  id: 3,
  path: '/films/Crime/Heat.1995.en.srt',
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
  path: '/films/Crime/Heat.1995.mkv',
  shelf: { name: 'Crime', path: '/films/Crime' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: 170 * 60_000,
  posterPath: '/data/posters/heat.webp',
  coverSource: 'none',
  accent: null,
  missing: false,
  details,
  tracks: [track],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

/** The sheet, open on a film the library holds. */
function open(changes: Partial<FilmView> = {}) {
  const showing = { ...film, ...changes };
  useLibrary.setState({ films: [showing], resumable: [], loaded: true });
  useSheet.setState({ filmId: showing.id });
  render(<FilmSheet />);
  return showing;
}

/** What one row of the file block says. */
function factOf(label: string): string {
  const term = screen.getByText(label);
  return term.nextElementSibling?.textContent ?? '';
}

describe('the film sheet', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSheet.setState({ filmId: null });
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
    useSettings.setState({ settings: DEFAULTS });
  });

  it('is not there at all until a film is opened', () => {
    useLibrary.setState({ films: [film], resumable: [], loaded: true });
    render(<FilmSheet />);

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  /*
   * The tile, the row, the billboard and the palette all come to this: a film
   * named to the store. Which controls do the naming is the library screen's
   * business, and is covered there.
   */
  it('opens on whichever film was named', () => {
    open();

    expect(screen.getByRole('dialog', { name: 'Heat' })).toBeInTheDocument();
    expect(screen.getByText('1995 · 2 hr 50 min · 1080p H.264')).toBeInTheDocument();
  });

  /*
   * Everything on the page was written when the film was scanned. Opening it
   * asks the back end for nothing at all, which is what the figure it has to
   * open in rests on.
   */
  it('asks the back end for nothing when it opens', () => {
    open();

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    for (const command of Object.values(ipc)) expect(command).not.toHaveBeenCalled();
  });

  /*
   * The only part of opening the page that could grow with the library is
   * finding the film in it, and that is a lookup. Two thousand films behind the
   * page cost what one does, which is what the figure it has to open in rests
   * on as much as the absence of a read does.
   */
  it('opens as quickly over two thousand films as over one', () => {
    const many = Array.from({ length: 2_000 }, (_, at) => ({
      ...film,
      id: at + 1,
      title: `Film ${String(at + 1).padStart(4, '0')}`,
    }));
    useLibrary.setState({ films: many, resumable: [], loaded: true });
    useSheet.setState({ filmId: 2_000 });

    const at = performance.now();
    render(<FilmSheet />);
    const took = performance.now() - at;

    expect(screen.getByRole('dialog', { name: 'Film 2000' })).toBeInTheDocument();

    // Loose enough not to fail on a busy machine, and tight enough to catch the
    // page starting to walk the library or read something as it opens.
    expect(took).toBeLessThan(500);
  });

  it('describes the file rather than the folder it is in', () => {
    open();

    expect(factOf('Container')).toBe('MKV');
    expect(factOf('Video')).toBe('H.264');
    expect(factOf('Resolution')).toBe('1920 × 1080');
    expect(factOf('Bit depth')).toBe('8-bit');
    expect(factOf('Frame rate')).toBe('23.976 fps');
    expect(factOf('Average bitrate')).toBe('12.4 Mb/s');
    expect(factOf('Size')).toBe('15.8 GB');
    expect(factOf('Path')).toBe('/films/Crime/Heat.1995.mkv');
  });

  /*
   * An MP4 is not parsed. What it does not say is left out rather than shown
   * empty, which is the rule the whole sheet is built on.
   */
  it('shows less for a film whose container was not read, and no empty rows', () => {
    open({
      path: '/films/Crime/Heat.1995.mp4',
      details: {
        container: 'MP4',
        sizeBytes: 4_100_000_000,
        averageBitrate: null,
        video: null,
        audio: [],
      },
    });

    expect(factOf('Container')).toBe('MP4');
    expect(screen.queryByText('Resolution')).not.toBeInTheDocument();
    expect(screen.queryByText('Frame rate')).not.toBeInTheDocument();
    expect(screen.getByText('Nothing recorded')).toBeInTheDocument();
  });

  it('lists the sound the film carries', () => {
    open();
    expect(screen.getByText('DTS-HD MA 5.1')).toBeInTheDocument();
  });

  it('says where each subtitle came from and how much of it there is', () => {
    open({
      tracks: [
        track,
        { ...track, id: 4, origin: 'stream', streamNumber: 2, language: 'fr', forced: true },
      ],
    });

    expect(screen.getByText('Beside the film · 1,402 lines')).toBeInTheDocument();
    expect(screen.getByText('Inside the film · 1,402 lines')).toBeInTheDocument();
    expect(screen.getByText('Forced')).toBeInTheDocument();
  });

  it('says plainly when a film has no subtitles at all', () => {
    open({ tracks: [] });

    expect(screen.getByText('None found')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /listen and line up/i })).not.toBeInTheDocument();
  });

  /*
   * Pictures are listed and named as unreadable, in the words the README uses.
   * A film whose only subtitles are pictures should say so rather than appear
   * to have none, and there is nothing to line up.
   */
  it('names a track of pictures as one it cannot read, and offers no alignment', () => {
    open({ tracks: [{ ...track, id: 9, form: 'pictures', origin: 'stream', cueCount: 0 }] });

    expect(screen.getByText('Inside the film')).toBeInTheDocument();
    expect(screen.getByText('Pictures, not text')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /listen and line up/i })).not.toBeInTheDocument();
  });

  it('says where a film was left, when it was left part way', () => {
    open({
      position: {
        positionMs: 112 * 60_000,
        durationMs: 170 * 60_000,
        finished: false,
        updatedAt: 5,
        progress: 112 / 170,
      },
    });

    expect(screen.getByText('58 min left · 66% watched')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /carry on/i })).toBeInTheDocument();
  });

  it('plays the film, and takes the sheet away as it goes', async () => {
    open();

    await userEvent.click(screen.getByRole('button', { name: /play/i }));

    expect(useNavigation.getState().route).toMatchObject({ screen: 'player', filmId: 7 });
    expect(useSheet.getState().filmId).toBeNull();
  });

  it('shows the film where it sits on the disk', async () => {
    open();

    await userEvent.click(screen.getByRole('button', { name: 'Show in folder' }));
    expect(ipc.showInFolder).toHaveBeenCalledWith(7);
  });

  /*
   * An unplugged drive. The film is still in the library and where it was
   * stopped is still on it, so this is a sentence rather than an error, and
   * neither of the two actions would do anything.
   */
  it('says a missing film is missing rather than offering to play it', () => {
    open({ missing: true });

    expect(screen.getByText(/the file is not where it was/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /play/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Show in folder' })).not.toBeInTheDocument();
  });

  it('closes on Escape', async () => {
    open();

    await userEvent.keyboard('{Escape}');
    expect(useSheet.getState().filmId).toBeNull();
  });

  it('closes when the scrim is pressed', async () => {
    open();

    // The one thing behind the panel, which is what a click outside lands on.
    const scrim = screen.getByRole('dialog').previousElementSibling;
    if (!(scrim instanceof HTMLElement)) throw new Error('the sheet has no scrim behind it');

    await userEvent.click(scrim);
    expect(useSheet.getState().filmId).toBeNull();
  });

  it('closes on the button in the corner', async () => {
    open();

    await userEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(useSheet.getState().filmId).toBeNull();
  });

  describe('lining the subtitles up from here', () => {
    it('offers it for a film whose subtitles can be read', async () => {
      open();

      await userEvent.click(screen.getByRole('button', { name: 'Listen and line up' }));
      expect(ipc.alignTrack).toHaveBeenCalledWith(3);
    });

    it('says what it is doing, and can be stopped', async () => {
      open();

      await userEvent.click(screen.getByRole('button', { name: 'Listen and line up' }));
      expect(screen.getByText('Listening to the film')).toBeInTheDocument();

      await userEvent.click(screen.getByRole('button', { name: 'Stop' }));
      expect(ipc.cancelAlignment).toHaveBeenCalled();
    });

    /*
     * A value somebody arrived at by ear is evidence about the file, from
     * somebody who watched it, so it is never replaced without being asked.
     */
    it('asks before it replaces a correction somebody made themselves', async () => {
      open({ tracks: [{ ...track, correction: { offsetMs: -450, rate: 1 } }] });

      await userEvent.click(screen.getByRole('button', { name: 'Listen and line up' }));
      expect(ipc.alignTrack).not.toHaveBeenCalled();
      expect(screen.getByText(/will replace that with what it measures/i)).toBeInTheDocument();

      await userEvent.click(screen.getByRole('button', { name: 'Measure anyway' }));
      expect(ipc.alignTrack).toHaveBeenCalledWith(3);
    });

    it('leaves the correction where it was when the answer is kept', async () => {
      open({ tracks: [{ ...track, correction: { offsetMs: -450, rate: 1 } }] });

      await userEvent.click(screen.getByRole('button', { name: 'Listen and line up' }));
      await userEvent.click(screen.getByRole('button', { name: 'Keep mine' }));

      expect(ipc.alignTrack).not.toHaveBeenCalled();
      expect(screen.getByRole('button', { name: 'Listen and line up' })).toBeInTheDocument();
    });

    it('says what it found, and offers to put back what it replaced', async () => {
      ipc.alignTrack.mockReturnValue(
        Promise.resolve({
          outcome: 'aligned',
          correction: { offsetMs: -1_250, rate: 1 },
          previous: { offsetMs: 0, rate: 1 },
          grounds: grounds(0.9),
          landing: landing(0.94),
          asWritten: landing(0.14),
          reference: { against: 'speech' },
        }),
      );
      open();

      await userEvent.click(screen.getByRole('button', { name: 'Listen and line up' }));

      await waitFor(() => {
        expect(screen.getByText('Lined up')).toBeInTheDocument();
      });
      expect(screen.getByRole('status')).toHaveTextContent(/The subtitles now run −1.25s\./);
      // The evidence for it, in the same sentence and the same words the timing
      // panel uses, since the two are read from one file.
      expect(screen.getByRole('status')).toHaveTextContent(
        /94 lines in a hundred land on the talking, against 14 before/,
      );
      // Including the grounds it rests on, which is the other half of what the
      // panel shows and would be the easier half to leave out here.
      expect(screen.getByRole('status')).toHaveTextContent(
        /18 of the 20 stretches of film it was measured across agreed, the middle one 40 milliseconds off/,
      );
      expect(screen.getByRole('button', { name: 'Put it back as written' })).toBeInTheDocument();
    });

    /*
     * Which track it measures is not a detail: a correction is written to one
     * track and the offer says nothing about which, so somebody with a film in
     * three languages cannot tell what is about to be moved.
     */
    it('names the subtitle it would measure', () => {
      open();

      expect(screen.getByText(/It measures English, which is the subtitle/)).toBeInTheDocument();
    });

    /*
     * The one this used to get wrong. A film whose subtitles have been turned
     * off is watched with none, and falling back to the first readable track
     * offered to measure a subtitle the player would not draw, which is a
     * correction written to a track nobody asked about and a measurement
     * nobody could check.
     */
    it('does not offer to measure a subtitle the player would not draw', () => {
      open({ subtitlesOff: true });

      expect(screen.queryByRole('button', { name: 'Listen and line up' })).not.toBeInTheDocument();
    });

    /*
     * The same check the player offers, from the page that has no player on it.
     * There is nothing here to watch a measurement land on, so taking it up
     * opens the film at the moment the check would have played.
     */
    it('opens the film at the busiest dialogue to watch a measurement land', async () => {
      ipc.alignTrack.mockReturnValue(
        Promise.resolve({
          outcome: 'aligned',
          correction: { offsetMs: -1_250, rate: 1 },
          previous: { offsetMs: 0, rate: 1 },
          grounds: grounds(0.9),
          landing: landing(0.94),
          asWritten: landing(0.14),
          reference: { against: 'speech' },
        }),
      );
      ipc.trackCues.mockResolvedValue(
        [0, 600_000, 602_000, 604_000].map((startMs, at) => ({
          index: at + 1,
          startMs,
          endMs: startMs + 1_500,
          text: 'line',
          position: null,
        })),
      );
      open();

      await userEvent.click(screen.getByRole('button', { name: 'Listen and line up' }));
      await waitFor(() => {
        expect(screen.getByText('Lined up')).toBeInTheDocument();
      });

      await userEvent.click(screen.getByRole('button', { name: CHECK.offer }));

      await waitFor(() => {
        expect(useNavigation.getState().route).toMatchObject({
          screen: 'player',
          filmId: 7,
          atMs: 600_000 - LEAD_MS,
        });
      });
      // And the page it was asked from is out of the way, since the answer is
      // in the film rather than on it.
      expect(useSheet.getState().filmId).toBeNull();
    });

    it('offers nothing to watch where the measurement was refused', async () => {
      ipc.alignTrack.mockReturnValue(Promise.resolve({ outcome: 'no-audio' }));
      open();

      await userEvent.click(screen.getByRole('button', { name: 'Listen and line up' }));
      await waitFor(() => {
        expect(screen.getByText('Nothing to listen to')).toBeInTheDocument();
      });

      expect(screen.queryByRole('button', { name: CHECK.offer })).not.toBeInTheDocument();
      expect(ipc.trackCues).not.toHaveBeenCalled();
    });

    /*
     * Every ending the command can report has a wording, including the ones
     * that changed nothing. Each of those says the nudge keys are still there.
     */
    it('says why it declined, without treating it as a fault', async () => {
      ipc.alignTrack.mockReturnValue(Promise.resolve({ outcome: 'unsupported', codec: 'AC-3' }));
      open();

      await userEvent.click(screen.getByRole('button', { name: 'Listen and line up' }));

      await waitFor(() => {
        expect(screen.getByText('Audio Subtext cannot read')).toBeInTheDocument();
      });
      expect(screen.getByRole('status')).toHaveTextContent(
        /AC-3, in a format Subtext has no decoder for/,
      );
      expect(screen.getByRole('status')).toHaveTextContent(/bracket keys still work/);
    });
  });
});
