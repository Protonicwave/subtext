import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { CueView, FilmView, TrackView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: { trackCues: vi.fn((): Promise<CueView[]> => Promise.resolve([])) },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});
// The URL a file is served from is the shell's business, and there is no shell
// under test.
vi.mock('@/shared/media/source', () => ({ sourceOf: (path: string) => `asset://${path}` }));

const { Masthead } = await import('./Masthead');
const { useSettings } = await import('@/shared/settings/useSettings');
const { DEFAULTS } = await import('@/shared/settings/schema');

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
  cueCount: 3,
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

const watching = {
  ...film,
  position: {
    positionMs: 122 * 60_000,
    durationMs: 170 * 60_000,
    finished: false,
    updatedAt: new Date(2026, 7, 19, 22, 10).getTime(),
    progress: 122 / 170,
  },
} satisfies FilmView;

const cues: CueView[] = [
  {
    index: 1,
    startMs: 100_000,
    endMs: 103_000,
    text: 'Told you I am never going back.',
    position: null,
  },
  {
    index: 2,
    startMs: 122 * 60_000 - 2_000,
    endMs: 122 * 60_000 + 1_000,
    text: 'We are sitting here like a couple of regular fellas.',
    position: null,
  },
  { index: 3, startMs: 9_000_000, endMs: 9_003_000, text: 'Do not waste my time.', position: null },
];

function show(shown: FilmView = watching) {
  const onPlay = vi.fn();
  const onOpen = vi.fn();
  render(<Masthead film={shown} onPlay={onPlay} onOpen={onOpen} />);
  return { onPlay, onOpen };
}

describe('the masthead', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    ipc.trackCues.mockResolvedValue(cues);
    useSettings.setState({ settings: DEFAULTS });
  });

  it('states where the reader stopped, and prints the line they stopped on', async () => {
    show();

    expect(screen.getByText(/you stopped here/i)).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1, name: 'Heat' })).toBeInTheDocument();
    expect(screen.getByText('48 min left of 2 hr 50 min')).toBeInTheDocument();
    expect(screen.getByText('Stopped yesterday')).toBeInTheDocument();
    expect(screen.getByText('English subtitles')).toBeInTheDocument();

    expect(
      await screen.findByText('We are sitting here like a couple of regular fellas.'),
    ).toBeInTheDocument();
    // The moment it was said, against the line.
    expect(screen.getByText('2:01:58')).toBeInTheDocument();
  });

  /*
   * The library is what somebody opened the window to see, and nothing on this
   * screen depends on the line. So the screen is drawn first and the dialogue
   * is asked for after it.
   */
  it('draws before it asks for the line', async () => {
    show();

    expect(screen.getByRole('heading', { level: 1, name: 'Heat' })).toBeInTheDocument();
    expect(ipc.trackCues).not.toHaveBeenCalled();

    await waitFor(() => {
      expect(ipc.trackCues).toHaveBeenCalledWith(3);
    });
  });

  it('says what a film nobody has started is, rather than where they got to', () => {
    show(film);

    expect(screen.getByText(/from your library/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^play$/i })).toBeInTheDocument();
    expect(screen.getByText('2 hr 50 min')).toBeInTheDocument();
    expect(screen.queryByText(/left of/)).not.toBeInTheDocument();
    expect(screen.queryByText(/^stopped/i)).not.toBeInTheDocument();
    expect(ipc.trackCues).not.toHaveBeenCalled();
  });

  it('prints no line for a film being watched without subtitles', async () => {
    show({ ...watching, subtitlesOff: true });

    expect(screen.getByText('Subtitles off')).toBeInTheDocument();
    await waitFor(() => {
      expect(ipc.trackCues).not.toHaveBeenCalled();
    });
  });

  it('prints no line for a film whose only subtitles are pictures', async () => {
    show({ ...watching, tracks: [{ ...track, form: 'pictures' }] });

    expect(screen.getByText('48 min left of 2 hr 50 min')).toBeInTheDocument();
    // Nothing said about subtitles either, since there are none to name.
    expect(screen.queryByText(/subtitles/i)).not.toBeInTheDocument();
    await waitFor(() => {
      expect(ipc.trackCues).not.toHaveBeenCalled();
    });
  });

  /*
   * The film is quiet where it was stopped, which is a line the masthead does
   * without rather than a note about why there is none.
   */
  it('says nothing where nothing was being said', async () => {
    ipc.trackCues.mockResolvedValue([cues[0]] as CueView[]);
    show();

    await waitFor(() => {
      expect(ipc.trackCues).toHaveBeenCalled();
    });
    expect(screen.queryByText(/told you i am never going back/i)).not.toBeInTheDocument();
  });

  it('carries on without a line when the dialogue cannot be read', async () => {
    ipc.trackCues.mockRejectedValue(new Error('the file has gone'));
    show();

    await waitFor(() => {
      expect(ipc.trackCues).toHaveBeenCalled();
    });
    expect(screen.getByRole('heading', { level: 1, name: 'Heat' })).toBeInTheDocument();
    expect(screen.getByText('48 min left of 2 hr 50 min')).toBeInTheDocument();
  });

  it('still describes a film whose file is missing', () => {
    show({ ...watching, missing: true });

    expect(screen.getByRole('heading', { level: 1, name: 'Heat' })).toBeInTheDocument();
    expect(screen.getByText('48 min left of 2 hr 50 min')).toBeInTheDocument();
  });

  it('offers to carry on with the film, and to read what it is', async () => {
    const { onPlay, onOpen } = show();

    await userEvent.click(screen.getByRole('button', { name: /carry on/i }));
    expect(onPlay).toHaveBeenCalledWith(watching);

    await userEvent.click(screen.getByRole('button', { name: 'Details' }));
    expect(onOpen).toHaveBeenCalledWith(watching);
  });
});
