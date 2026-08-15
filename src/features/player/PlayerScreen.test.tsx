import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as BindingsModule from '@/shared/ipc/bindings';
import type * as ClientModule from '@/shared/ipc/client';
import type { CueView, FilmView, TrackView } from '@/shared/ipc/bindings';
import { opens, positionOf, pretendMediaWorks, reaches, refuses } from '@/test/media';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    trackCues: vi.fn(() => Promise.resolve<CueView[]>([])),
    // Keeping the value is covered where the hook is tested. Refusing it here
    // leaves what was nudged on screen, which keeps this file about the picture
    // rather than about the database.
    setTrackCorrection: vi.fn(() => Promise.reject(new Error('not under test'))),
    // Refused unless a test says otherwise, so that choosing a track is only
    // answered where the answer is what the test is about.
    setFilmTrack: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
    // Reading a film takes a while and this one never finishes, which is what
    // lets a test look at the panel while the work is going on.
    alignTrack: vi.fn(
      () =>
        new Promise<never>(() => {
          /* still reading */
        }),
    ),
    cancelAlignment: vi.fn(() => Promise.resolve(null)),
    savePosition: vi.fn(() => Promise.resolve(null)),
    listFolders: vi.fn(() => Promise.resolve([])),
    listLibrary: vi.fn(() => Promise.resolve([])),
    continueWatching: vi.fn(() => Promise.resolve([])),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});
// Nothing here asks for an alignment, and the real emitter would go looking for
// a window to subscribe through. Only the one the player listens to is stood
// in for, so the rest of the bindings stay as they are.
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
// under test. The screen only cares that it has one.
vi.mock('@/shared/media/source', () => ({
  sourceOf: (path: string) => `asset://${path}`,
  streamOf: (path: string) => `stream://${path}`,
}));

const { PlayerScreen } = await import('./PlayerScreen');
const { useLibrary } = await import('./../library/useLibrary');
const { useNavigation } = await import('@/app/routes');
const { DEFAULTS } = await import('@/shared/settings/schema');
const { useSettings } = await import('@/shared/settings/useSettings');

pretendMediaWorks();

const RUNS = 10_260_000;

/**
 * How far before its written start a line goes up, and therefore how far before
 * it going anywhere by line lands.
 *
 * Stepping goes to the moment the line is on screen from rather than to the
 * moment it was written at, because they are the same timeline: one answer to
 * where a line is, shared by everything that asks. Landing a little before the
 * first syllable is also what somebody who missed the line wanted.
 */
const LEAD = DEFAULTS.subtitleLeadInMs;

const english = {
  id: 3,
  path: '/films/Heat.1995.srt',
  language: 'en',
  forced: false,
  hearingImpaired: false,
  origin: 'sidecar' as const,
  streamNumber: 0,
  form: 'text' as const,
  matchKind: 'exact' as const,
  correction: { offsetMs: 0, rate: 1 },
  cueCount: 2,
} satisfies TrackView;

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  shelf: { name: 'films', path: '/films' },
  title: 'Heat',
  year: 1995,
  durationMs: RUNS,
  posterPath: null,
  accent: null,
  missing: false,
  details: null,
  tracks: [english],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

function open(changes: Partial<FilmView> = {}) {
  useLibrary.setState({ folders: [], films: [{ ...film, ...changes }], resumable: [] });
  const view = render(<PlayerScreen filmId={7} />);

  return { view, video: playing };
}

/**
 * Waits for the film's dialogue to have been read.
 *
 * Nothing on screen says so directly: a line is only drawn at the moment it is
 * spoken. The line controls are the honest signal, since they are enabled by
 * there being lines to step between.
 */
async function dialogueArrives(): Promise<void> {
  await waitFor(() => {
    expect(screen.getByRole('button', { name: /next line/i })).toBeEnabled();
  });
}

/** The element on screen, or a failed test rather than a null dereference. */
function playing(): HTMLVideoElement {
  const video = document.querySelector('video');
  if (video === null) throw new Error('the player should be showing a video element');
  return video;
}

describe('playing a film', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useNavigation.setState({ route: { screen: 'player', filmId: 7 }, previous: null });
    useSettings.setState({ settings: DEFAULTS, problem: null });
  });

  it('points the element at the file through the protocol that serves it', () => {
    const { video } = open();

    expect(video()).toHaveAttribute('src', 'stream:///films/Heat.1995.mkv');
  });

  it('starts a film nobody has watched from the beginning', () => {
    const { video } = open();
    opens(video(), RUNS);

    expect(positionOf(video())).toBe(0);
    expect(video().paused).toBe(false);
  });

  it('carries on a little before where it was left', () => {
    const { video } = open({
      position: {
        positionMs: 600_000,
        durationMs: RUNS,
        finished: false,
        updatedAt: 0,
        progress: null,
      },
    });
    opens(video(), RUNS);

    expect(positionOf(video())).toBe(595_000);
  });

  it('writes down where it got to on the way out', async () => {
    const { view, video } = open();
    opens(video(), RUNS);
    reaches(video(), 630_000);

    view.unmount();

    await waitFor(() => {
      expect(ipc.savePosition).toHaveBeenCalledWith(7, 630_000, RUNS, false);
    });
  });

  it('counts a film watched to the end as finished', async () => {
    const { view, video } = open();
    opens(video(), RUNS);
    reaches(video(), RUNS);

    view.unmount();

    await waitFor(() => {
      expect(ipc.savePosition).toHaveBeenCalledWith(7, RUNS, RUNS, true);
    });
  });

  it('does not remember a film that was opened and closed again', () => {
    const { view, video } = open();
    opens(video(), RUNS);

    view.unmount();

    expect(ipc.savePosition).not.toHaveBeenCalled();
  });

  it('says that a film is on its way while it is still opening', () => {
    const { video } = open();
    expect(screen.getByRole('status', { name: /opening the film/i })).toBeInTheDocument();

    opens(video(), RUNS);
    expect(screen.queryByRole('status', { name: /opening the film/i })).not.toBeInTheDocument();
  });

  it('says what is likely wrong with a file it cannot decode', () => {
    const { video } = open();
    refuses(video(), 3);

    expect(screen.getByRole('alert')).toHaveTextContent(/will not play/i);
    expect(screen.getByText(/HEVC video or DTS audio/)).toBeInTheDocument();
    // Nothing to control, so nothing that looks controllable.
    expect(screen.queryByRole('button', { name: /pause/i })).not.toBeInTheDocument();
  });

  it('keeps the position of a film whose file has gone, and says where it was', () => {
    open({ missing: true });

    expect(screen.getByText(/not where it was/i)).toBeInTheDocument();
    expect(screen.getByText('/films/Heat.1995.mkv')).toBeInTheDocument();
    expect(document.querySelector('video')).toBeNull();
  });

  it('says so when the film is no longer in the library at all', () => {
    useLibrary.setState({ folders: [], films: [], resumable: [] });
    render(<PlayerScreen filmId={7} />);

    expect(screen.getByText(/no longer in the library/i)).toBeInTheDocument();
  });

  it('is watched with the keyboard as well as with the pointer', async () => {
    const { video } = open();
    opens(video(), RUNS);
    reaches(video(), 600_000);

    await userEvent.keyboard('{ }');
    expect(video().paused).toBe(true);

    await userEvent.keyboard('{ArrowRight}');
    expect(positionOf(video())).toBe(610_000);

    await userEvent.keyboard('{ArrowLeft}{ArrowLeft}');
    expect(positionOf(video())).toBe(590_000);

    await userEvent.keyboard('m');
    expect(video().muted).toBe(true);
  });

  it('lands the arrow keys on dialogue rather than on a fixed ten seconds', async () => {
    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 60_000, endMs: 64_000, text: 'I take scores.', position: null },
      { index: 2, startMs: 600_000, endMs: 604_000, text: 'I rob banks.', position: null },
    ]);

    const { video } = open();
    opens(video(), RUNS);
    await dialogueArrives();
    reaches(video(), 300_000);

    await userEvent.keyboard('{ArrowRight}');
    expect(positionOf(video())).toBe(600_000 - LEAD);

    await userEvent.keyboard('{ArrowLeft}');
    expect(positionOf(video())).toBe(60_000 - LEAD);
  });

  it('steps by line from the control bar', async () => {
    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 60_000, endMs: 64_000, text: 'I take scores.', position: null },
    ]);

    const { video } = open();
    opens(video(), RUNS);
    await dialogueArrives();

    await userEvent.click(screen.getByRole('button', { name: /next line/i }));
    expect(positionOf(video())).toBe(60_000 - LEAD);
  });

  it('draws the line being spoken at the moment it is spoken', async () => {
    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 1_000, endMs: 4_000, text: 'I take scores.', position: null },
    ]);

    const { video } = open();
    opens(video(), RUNS);
    reaches(video(), 2_000);

    expect(await screen.findByText('I take scores.')).toBeInTheDocument();
  });

  it('puts a line up a little early and holds a short one long enough to read', async () => {
    // A quarter of a second as the file wrote it, which is not long enough for
    // anybody, and it is asked for a moment before it was written to begin.
    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 1_000, endMs: 1_250, text: 'Not really.', position: null },
    ]);
    useSettings.setState({
      settings: { ...DEFAULTS, subtitleLeadInMs: 100, subtitleMinimumMs: 1_000 },
      problem: null,
    });

    const { video } = open();
    opens(video(), RUNS);
    reaches(video(), 950);

    expect(await screen.findByText('Not really.')).toBeInTheDocument();

    // Gone once it has been up for as long as it was promised, and back if the
    // film is put anywhere inside that. Looked at in this order because each
    // assertion is then something the frame loop had to do, rather than a line
    // that happened to still be there from the assertion before.
    reaches(video(), 1_950);
    await waitFor(() => {
      expect(screen.queryByText('Not really.')).not.toBeInTheDocument();
    });

    // Past the end the file gave it, which is the whole point of the minimum.
    reaches(video(), 1_400);
    expect(await screen.findByText('Not really.')).toBeInTheDocument();
  });

  it('moves the subtitles against the film while the value is being felt for', async () => {
    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 5_000, endMs: 7_000, text: 'I take scores.', position: null },
    ]);
    useSettings.setState({
      settings: { ...DEFAULTS, subtitleLeadInMs: 0, subtitleMinimumMs: 0 },
      problem: null,
    });

    const { video } = open();
    opens(video(), RUNS);
    // Half a second past the end of the line, where nothing is on screen.
    reaches(video(), 7_500);
    expect(screen.queryByText('I take scores.')).not.toBeInTheDocument();

    // Twenty presses of a fiftieth of a second, which is the line's own second
    // of lateness. Nothing is written down yet, and the picture has already
    // caught up, which is what makes this findable by ear.
    const typing = userEvent.setup();
    for (let press = 0; press < 20; press += 1) {
      await typing.keyboard(']');
    }

    expect(await screen.findByText('I take scores.')).toBeInTheDocument();
    expect(screen.getByText(/Subtitles \+1\.00s/)).toBeInTheDocument();
  });

  /*
   * A forced track is a handful of lines over the signs. Opening on one is the
   * mistake taking whichever sorted first used to make, and it looks from the
   * outside like a film that has almost no subtitles.
   */
  it('opens on the full subtitle rather than on the forced one', async () => {
    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 1_000, endMs: 4_000, text: 'I take scores.', position: null },
    ]);

    const { video } = open({
      tracks: [
        { ...english, id: 3, path: '/films/Heat.forced.srt', forced: true },
        { ...english, id: 4 },
      ],
    });
    opens(video(), RUNS);

    await dialogueArrives();
    expect(ipc.trackCues).toHaveBeenCalledWith(4);
  });

  it('swaps the dialogue for another track without stopping the film', async () => {
    const tracks = [
      { ...english, id: 3 },
      { ...english, id: 4, language: 'fr', path: '/films/Heat.fr.srt' },
    ];
    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 1_000, endMs: 4_000, text: 'I take scores.', position: null },
    ]);
    ipc.setFilmTrack.mockResolvedValueOnce({ ...film, tracks, chosenTrackId: 4 });

    const { video } = open({ tracks });
    opens(video(), RUNS);
    reaches(video(), 2_000);
    expect(await screen.findByText('I take scores.')).toBeInTheDocument();

    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 1_000, endMs: 4_000, text: 'Je fais des casses.', position: null },
    ]);

    await userEvent.keyboard('c');
    await userEvent.click(await screen.findByRole('radio', { name: /French/ }));

    expect(ipc.setFilmTrack).toHaveBeenCalledWith(7, 4);
    // The same moment of the same film, saying what the other track says there.
    expect(await screen.findByText('Je fais des casses.')).toBeInTheDocument();
    expect(screen.queryByText('I take scores.')).not.toBeInTheDocument();
    // The picture was never touched: a different track is a different array of
    // cues and nothing else.
    expect(video().paused).toBe(false);
    expect(positionOf(video())).toBe(2_000);
  });

  /*
   * The whole point of doing it from here: somebody notices the subtitles are
   * out while they are watching, and one key puts it right without stopping
   * what they are watching.
   */
  it('measures the subtitle against the film without interrupting it', async () => {
    const { video } = open();
    opens(video(), RUNS);
    reaches(video(), 60_000);

    await userEvent.keyboard('a');

    expect(ipc.alignTrack).toHaveBeenCalledWith(3);
    // The panel comes up with it, because everything it has to say afterwards
    // is said there.
    expect(await screen.findByText('Listening to the film')).toBeInTheDocument();
    expect(video().paused).toBe(false);
    expect(positionOf(video())).toBe(60_000);
  });

  it('stops the reading when it is asked to, from the panel', async () => {
    const { video } = open();
    opens(video(), RUNS);

    await userEvent.keyboard('a');
    await userEvent.click(await screen.findByRole('button', { name: 'Stop' }));

    expect(ipc.cancelAlignment).toHaveBeenCalled();
  });

  it('leaves the align key alone for a film with no subtitle to measure', async () => {
    const { video } = open({ tracks: [] });
    opens(video(), RUNS);

    await userEvent.keyboard('a');

    expect(ipc.alignTrack).not.toHaveBeenCalled();
  });

  it('leaves the nudge keys alone for a film with no subtitle to move', async () => {
    const { video } = open({ tracks: [] });
    opens(video(), RUNS);

    const typing = userEvent.setup();
    await typing.keyboard(']');

    expect(screen.queryByText(/Subtitles/)).not.toBeInTheDocument();
  });
});
