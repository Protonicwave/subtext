import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, ScanProgressed, ScanSummary, TrackView } from '@/shared/ipc/bindings';

const { ipc, emitters, emit } = vi.hoisted(() => {
  const listeners = new Map<string, (payload: unknown) => void>();
  const emitter = (name: string) => ({
    listen: (callback: (event: { payload: unknown }) => void) => {
      listeners.set(name, (payload) => {
        callback({ payload });
      });
      return Promise.resolve(() => listeners.delete(name));
    },
  });

  return {
    ipc: {
      chooseSubtitle: vi.fn(),
      attachSubtitle: vi.fn(),
      listFolders: vi.fn(() => Promise.resolve([])),
      listLibrary: vi.fn(() => Promise.resolve([])),
    },
    emitters: {
      scanProgressed: emitter('progressed'),
      scanFinished: emitter('finished'),
      scanFailed: emitter('failed'),
    },
    emit: (name: string, payload: unknown) => {
      const listener = listeners.get(name);
      if (!listener) throw new Error(`nothing is listening for ${name}`);
      listener(payload);
    },
  };
});

// The client builds itself from the commands when it loads, so the mocked
// bindings have to offer the list even though every call is stood in for below.
vi.mock('@/shared/ipc/bindings', () => ({ commands: {}, events: emitters }));
vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { ImportFlow } = await import('./ImportFlow');
const { useImport } = await import('./useImport');
const { useLibrary } = await import('@/features/library/useLibrary');

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
  path: '/films/Heat.1995.1080p.BluRay.mkv',
  shelf: { name: 'films', path: '/films' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: null,
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

const unpaired = {
  ...film,
  id: 8,
  path: '/films/Sicario.2015.mkv',
  title: 'Sicario',
  year: 2015,
  tracks: [],
} satisfies FilmView;

/** A track the film carries itself, which was never paired with anything. */
function inside(id: number, changes: Partial<TrackView> = {}): TrackView {
  return {
    ...track,
    id,
    path: '/films/Sicario.2015.mkv',
    origin: 'stream',
    streamNumber: id,
    cueCount: 0,
    ...changes,
  };
}

const summary = {
  folderId: 1,
  filmsFound: 2,
  subtitlesFound: 1,
  filmsPaired: 1,
  subtitlesRead: 1,
  cuesIndexed: 1_402,
  filmsMissing: 0,
  coversChanged: 0,
  unpairedSubtitles: [],
  filmsWithoutSubtitles: ['/films/Sicario.2015.mkv'],
  unreadable: [],
  warnings: [],
} satisfies ScanSummary;

const progress = {
  folderId: 1,
  stage: 'indexing',
  filesSeen: 3,
  filmsFound: 2,
  subtitlesFound: 1,
  filmsPaired: 1,
  subtitlesToRead: 1,
  subtitlesRead: 0,
  filmsToRead: 2,
  filmsRead: 1,
  cuesIndexed: 640,
  fractionRead: 0.5,
} satisfies ScanProgressed;

/** An import that a person started, which is the only kind that shows itself. */
function asked() {
  useImport.setState({ stage: 'indexing', asked: true });
}

describe('importing a folder', () => {
  // A scan finishing has the library read again. What that reads is the library
  // store's business and is tested there, so here it is stood in for and the
  // films are whatever the test says they are.
  const refresh = vi.fn(() => Promise.resolve());

  beforeEach(() => {
    vi.clearAllMocks();
    refresh.mockReset();
    refresh.mockResolvedValue(undefined);
    useImport.setState({
      stage: 'idle',
      progress: null,
      summaries: [],
      problem: null,
      asked: false,
    });
    useLibrary.setState({ films: [film, unpaired], folders: [], refresh });
  });

  it('follows the scan as it reports itself', async () => {
    render(<ImportFlow />);
    asked();

    emit('progressed', progress);

    expect(await screen.findByText(/640 lines indexed/)).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: /indexing/i })).toHaveAttribute(
      'aria-valuenow',
      '50',
    );
  });

  it('shows every film with the subtitle it was paired to', async () => {
    render(<ImportFlow />);
    asked();

    emit('finished', [summary]);

    const sheet = await screen.findByRole('dialog', { name: 'What was found' });

    expect(sheet).toHaveAttribute('open');
    expect(screen.getByText('Heat.1995.1080p.BluRay.mkv')).toBeInTheDocument();
    expect(screen.getByText('Heat.1995.en.srt')).toBeInTheDocument();
    expect(screen.getByText('Matched')).toBeInTheDocument();
    expect(sheet).toHaveTextContent('2 films · 1 with subtitles · 1 without');
  });

  it('lists the tracks a film carries inside it', async () => {
    useLibrary.setState({
      films: [
        {
          ...unpaired,
          tracks: [
            inside(2),
            inside(3, { language: 'fr' }),
            inside(4, { form: 'pictures', language: null }),
          ],
        },
      ],
      folders: [],
      refresh,
    });

    render(<ImportFlow />);
    asked();
    emit('finished', [summary]);

    const sheet = await screen.findByRole('dialog', { name: 'What was found' });

    // A film with no file beside it is not a film with no subtitles, and the
    // row has to say which of the two it is.
    expect(sheet).toHaveTextContent('1 film · 1 with subtitles');
    expect(screen.getByText('English')).toBeInTheDocument();
    expect(screen.getByText('French')).toBeInTheDocument();

    // The one nothing can be done with is named rather than left off, so that
    // a film whose subtitles never appear says why. It says nothing about what
    // language it is in, so the number the film knows it by has to do.
    expect(screen.getByText('Track 4')).toBeInTheDocument();
    expect(screen.getByText('Pictures, not text')).toBeInTheDocument();
    expect(screen.getAllByText('Inside the film')).toHaveLength(2);
  });

  it('counts a film carrying only pictures as having no subtitles', async () => {
    useLibrary.setState({
      films: [{ ...unpaired, tracks: [inside(2, { form: 'pictures' })] }],
      folders: [],
      refresh,
    });

    render(<ImportFlow />);
    asked();
    emit('finished', [summary]);

    const sheet = await screen.findByRole('dialog', { name: 'What was found' });
    expect(sheet).toHaveTextContent('1 film · 0 with subtitles · 1 without');
  });

  it('says when a match was only a close one', async () => {
    useLibrary.setState({
      films: [{ ...film, tracks: [{ ...track, matchKind: 'approximate' }] }],
    });
    render(<ImportFlow />);
    asked();

    emit('finished', [summary]);

    expect(await screen.findByText('Close match')).toBeInTheDocument();
  });

  it('offers to attach a subtitle to a film that has none', async () => {
    ipc.chooseSubtitle.mockResolvedValue('/downloads/sicario english.srt');
    ipc.attachSubtitle.mockResolvedValue({ ...track, matchKind: 'by-hand' });

    render(<ImportFlow />);
    asked();
    emit('finished', [summary]);

    expect(await screen.findByText('No subtitle file found beside it')).toBeInTheDocument();

    // From here on, reading the library finds the file attached.
    refresh.mockImplementation(() => {
      useLibrary.setState({
        films: [
          film,
          {
            ...unpaired,
            tracks: [{ ...track, path: '/downloads/sicario english.srt', matchKind: 'by-hand' }],
          },
        ],
      });
      return Promise.resolve();
    });

    await userEvent.click(screen.getByRole('button', { name: 'Attach .srt' }));

    await waitFor(() => {
      expect(ipc.attachSubtitle).toHaveBeenCalledWith(8, '/downloads/sicario english.srt');
    });
    expect(await screen.findByText('Attached')).toBeInTheDocument();
  });

  it('leaves the row alone when the picker was dismissed', async () => {
    ipc.chooseSubtitle.mockResolvedValue(null);

    render(<ImportFlow />);
    asked();
    emit('finished', [summary]);

    await userEvent.click(await screen.findByRole('button', { name: 'Attach .srt' }));

    expect(ipc.attachSubtitle).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Attach .srt' })).toBeEnabled();
  });

  it('says why a subtitle could not be attached', async () => {
    ipc.chooseSubtitle.mockResolvedValue('/downloads/not a subtitle.srt');
    ipc.attachSubtitle.mockRejectedValue(new Error('that film is no longer in the library'));

    render(<ImportFlow />);
    asked();
    emit('finished', [summary]);

    await userEvent.click(await screen.findByRole('button', { name: 'Attach .srt' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'that film is no longer in the library',
    );
  });

  it('closes when it has been read', async () => {
    render(<ImportFlow />);
    asked();
    emit('finished', [summary]);

    await userEvent.click(await screen.findByRole('button', { name: 'Done' }));

    expect(useImport.getState().stage).toBe('idle');
  });

  it('cannot be dismissed while the scan is still running', async () => {
    render(<ImportFlow />);
    asked();
    emit('progressed', progress);
    await screen.findByText(/lines indexed/);

    await userEvent.keyboard('{Escape}');

    expect(useImport.getState().stage).toBe('indexing');
  });
});
