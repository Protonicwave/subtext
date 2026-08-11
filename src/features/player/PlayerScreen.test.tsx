import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { CueView, FilmView } from '@/shared/ipc/bindings';
import { opens, positionOf, pretendMediaWorks, reaches, refuses } from '@/test/media';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    trackCues: vi.fn(() => Promise.resolve<CueView[]>([])),
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
// The URL a file is served from is the shell's business, and there is no shell
// under test. The screen only cares that it has one.
vi.mock('@/shared/media/source', () => ({
  sourceOf: (path: string) => `asset://${path}`,
  streamOf: (path: string) => `stream://${path}`,
}));

const { PlayerScreen } = await import('./PlayerScreen');
const { useLibrary } = await import('./../library/useLibrary');
const { useNavigation } = await import('@/app/routes');

pretendMediaWorks();

const RUNS = 10_260_000;

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  title: 'Heat',
  year: 1995,
  durationMs: RUNS,
  posterPath: null,
  accent: null,
  missing: false,
  tracks: [
    {
      id: 3,
      path: '/films/Heat.1995.srt',
      language: 'en',
      forced: false,
      hearingImpaired: false,
      matchKind: 'exact' as const,
      cueCount: 2,
    },
  ],
  position: null,
} satisfies FilmView;

function open(changes: Partial<FilmView> = {}) {
  useLibrary.setState({ folders: [], films: [{ ...film, ...changes }], resumable: [] });
  const view = render(<PlayerScreen filmId={7} />);

  return { view, video: playing };
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

  it('draws the line being spoken at the moment it is spoken', async () => {
    ipc.trackCues.mockResolvedValueOnce([
      { index: 1, startMs: 1_000, endMs: 4_000, text: 'I take scores.', position: null },
    ]);

    const { video } = open();
    opens(video(), RUNS);
    reaches(video(), 2_000);

    expect(await screen.findByText('I take scores.')).toBeInTheDocument();
  });
});
