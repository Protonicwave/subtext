import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, TrackView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: { setTrackCorrection: vi.fn() },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { useLibrary } = await import('@/features/library/useLibrary');
const { STEP_MS, offsetLabel, useSync } = await import('./useSync');

/** The track being watched, as the player hands it over. */
function trackWith(offsetMs: number, rate = 1): TrackView {
  return {
    id: 3,
    path: '/films/Heat.1995.srt',
    language: 'en',
    forced: false,
    hearingImpaired: false,
    origin: 'sidecar',
    streamNumber: 0,
    form: 'text',
    matchKind: 'exact',
    cueCount: 1_200,
    correction: { offsetMs, rate },
  };
}

/** The film the write comes back as, which is what the library then holds. */
function filmWith(offsetMs: number, rate = 1): FilmView {
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
    tracks: [trackWith(offsetMs, rate)],
    chosenTrackId: null,
    subtitlesOff: false,
    position: null,
  };
}

beforeEach(() => {
  vi.useFakeTimers();
  ipc.setTrackCorrection.mockReset().mockResolvedValue(filmWith(0));
  useLibrary.setState({ films: [filmWith(0)], resumable: [] });
});

afterEach(() => {
  vi.useRealTimers();
});

describe('putting a subtitle back in step by ear', () => {
  it('shows the value being felt for before anything is written down', () => {
    const { result } = renderHook(() => useSync(trackWith(0)));

    act(() => {
      result.current.nudge(STEP_MS);
    });

    expect(result.current.offsetMs).toBe(STEP_MS);
    expect(result.current.pending).toBe(true);
    expect(ipc.setTrackCorrection).not.toHaveBeenCalled();
  });

  it('counts a run of presses as one adjustment and writes it once', async () => {
    const { result } = renderHook(() => useSync(trackWith(0)));

    for (let press = 0; press < 4; press += 1) {
      act(() => {
        result.current.nudge(STEP_MS);
      });
      // Well inside the time it waits, so each press puts the write off again.
      act(() => {
        vi.advanceTimersByTime(300);
      });
    }

    expect(ipc.setTrackCorrection).not.toHaveBeenCalled();

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    expect(ipc.setTrackCorrection).toHaveBeenCalledTimes(1);
    expect(ipc.setTrackCorrection).toHaveBeenCalledWith(3, {
      offsetMs: STEP_MS * 4,
      rate: 1,
    });
    expect(result.current.pending).toBe(false);
  });

  it('carries on from the value already stored', () => {
    const { result } = renderHook(() => useSync(trackWith(-2_000)));
    expect(result.current.offsetMs).toBe(-2_000);

    act(() => {
      result.current.nudge(-STEP_MS);
    });

    expect(result.current.offsetMs).toBe(-2_000 - STEP_MS);
  });

  it('puts the film the back end returns back into the library', async () => {
    const corrected = filmWith(400);
    ipc.setTrackCorrection.mockResolvedValue(corrected);

    const { result } = renderHook(() => useSync(trackWith(0)));
    act(() => {
      result.current.nudge(400);
    });
    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    // Read back rather than assumed, so what the player and the library hold is
    // what the database kept.
    expect(useLibrary.getState().films[0]?.tracks[0]?.correction.offsetMs).toBe(400);
  });

  it('writes a rate straight away, since it is chosen rather than felt for', async () => {
    const { result } = renderHook(() => useSync(trackWith(250)));

    await act(async () => {
      result.current.setRate(1.042_709_376);
      await Promise.resolve();
    });

    expect(ipc.setTrackCorrection).toHaveBeenCalledWith(3, {
      offsetMs: 250,
      rate: 1.042_709_376,
    });
  });

  it('goes back to the file exactly as it was written', async () => {
    const { result } = renderHook(() => useSync(trackWith(-1_500, 1.05)));

    await act(async () => {
      result.current.reset();
      await Promise.resolve();
    });

    expect(ipc.setTrackCorrection).toHaveBeenCalledWith(3, { offsetMs: 0, rate: 1 });
  });

  it('keeps the value on screen when it could not be written down', async () => {
    ipc.setTrackCorrection.mockRejectedValue(new Error('the database is locked'));

    const { result } = renderHook(() => useSync(trackWith(0)));
    act(() => {
      result.current.nudge(STEP_MS);
    });
    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    // The film reads right for this sitting, which is more use than putting the
    // subtitles back out of step to make a point about the database.
    expect(result.current.offsetMs).toBe(STEP_MS);
  });

  it('offers nothing for a film with no subtitle to move', () => {
    const { result } = renderHook(() => useSync(null));

    expect(result.current.available).toBe(false);
    act(() => {
      result.current.nudge(STEP_MS);
    });

    expect(result.current.offsetMs).toBe(0);
    expect(ipc.setTrackCorrection).not.toHaveBeenCalled();
  });
});

describe('saying what the offset is', () => {
  it('always says which way, so that nought reads as nought', () => {
    expect(offsetLabel(0)).toBe('as written');
    expect(offsetLabel(250)).toBe('+0.25s');
    expect(offsetLabel(-1_500)).toBe('−1.50s');
  });
});
