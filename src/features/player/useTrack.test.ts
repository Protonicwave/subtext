import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, Id, TrackView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: { setFilmTrack: vi.fn() },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { useLibrary } = await import('@/features/library/useLibrary');
const { DEFAULTS } = await import('@/shared/settings/schema');
const { useSettings } = await import('@/shared/settings/useSettings');
const { useTrack } = await import('./useTrack');

function track(id: Id, changes: Partial<TrackView> = {}): TrackView {
  return {
    id,
    path: `/films/Heat.${String(id)}.srt`,
    language: 'en',
    forced: false,
    hearingImpaired: false,
    origin: 'sidecar',
    streamNumber: 0,
    form: 'text',
    matchKind: 'exact',
    cueCount: 1_200,
    correction: { offsetMs: 0, rate: 1 },
    ...changes,
  };
}

function film(tracks: TrackView[], changes: Partial<FilmView> = {}): FilmView {
  return {
    id: 7,
    folderId: 1,
    path: '/films/Heat.1995.mkv',
    shelf: { name: 'films', path: '/films' },
    title: 'Heat',
    year: 1995,
    addedAt: Date.UTC(2026, 7, 12),
    durationMs: null,
    posterPath: null,
    accent: null,
    missing: false,
    details: null,
    tracks,
    chosenTrackId: null,
    subtitlesOff: false,
    position: null,
    ...changes,
  };
}

beforeEach(() => {
  useSettings.setState({ settings: DEFAULTS, problem: null });
  useLibrary.setState({ films: [], resumable: [], problem: null });
  ipc.setFilmTrack.mockReset();
});

describe('which subtitle a film is watched with', () => {
  it('is picked by the rule until somebody says otherwise', () => {
    const forced = track(1, { forced: true });
    const full = track(2);
    const { result } = renderHook(() => useTrack(film([forced, full])));

    expect(result.current.active).toBe(full);
    expect(result.current.available).toBe(true);
    expect(result.current.off).toBe(false);
  });

  it('follows the preferred language from the settings', () => {
    useSettings.setState({
      settings: { ...DEFAULTS, subtitleLanguage: 'fr' },
      problem: null,
    });
    const english = track(1);
    const french = track(2, { language: 'fr' });

    const { result } = renderHook(() => useTrack(film([english, french])));
    expect(result.current.active).toBe(french);
  });

  it('is nothing at all for a film with no subtitles', () => {
    const { result } = renderHook(() => useTrack(film([])));

    expect(result.current.active).toBeNull();
    expect(result.current.available).toBe(false);
  });

  it('writes the choice down and takes the film back from the database', async () => {
    const english = track(1);
    const french = track(2, { language: 'fr' });
    const opened = film([english, french]);
    const chosen = film([english, french], { chosenTrackId: 2 });
    ipc.setFilmTrack.mockResolvedValue(chosen);
    useLibrary.setState({ films: [opened], resumable: [] });

    const { result } = renderHook(() => useTrack(opened));
    act(() => {
      result.current.choose(french);
    });

    expect(ipc.setFilmTrack).toHaveBeenCalledWith(7, 2);
    // Read back rather than assumed, so what the player holds is what was kept.
    await waitFor(() => {
      expect(useLibrary.getState().films[0]?.chosenTrackId).toBe(2);
    });
  });

  it('turns the subtitles off without that being the same as having none', async () => {
    const english = track(1);
    const opened = film([english]);
    ipc.setFilmTrack.mockResolvedValue(film([english], { subtitlesOff: true }));
    useLibrary.setState({ films: [opened], resumable: [] });

    const { result } = renderHook(() => useTrack(opened));
    act(() => {
      result.current.choose(null);
    });

    expect(ipc.setFilmTrack).toHaveBeenCalledWith(7, null);
    await waitFor(() => {
      expect(useLibrary.getState().films[0]?.subtitlesOff).toBe(true);
    });

    // The film still has the track. It is not being read, which is a different
    // thing from there being nothing to read.
    const { result: after } = renderHook(() => useTrack(film([english], { subtitlesOff: true })));
    expect(after.current.active).toBeNull();
    expect(after.current.available).toBe(true);
  });

  it('leaves the film on the track it was reading when the write fails', async () => {
    const english = track(1);
    const french = track(2, { language: 'fr' });
    const opened = film([english, french]);
    ipc.setFilmTrack.mockRejectedValue(new Error('the database is locked'));
    useLibrary.setState({ films: [opened], resumable: [] });

    const { result } = renderHook(() => useTrack(opened));
    act(() => {
      result.current.choose(french);
    });

    await waitFor(() => {
      expect(ipc.setFilmTrack).toHaveBeenCalled();
    });
    expect(useLibrary.getState().films[0]?.chosenTrackId).toBeNull();
    expect(result.current.active).toBe(english);
  });
});
