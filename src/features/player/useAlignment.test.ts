import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { AlignProgressed, AlignmentView, TrackView } from '@/shared/ipc/bindings';
import type { Sync } from './useSync';

const { ipc, emitters, emit } = vi.hoisted(() => {
  const listeners = new Set<(payload: unknown) => void>();

  return {
    ipc: {
      alignTrack: vi.fn(),
      cancelAlignment: vi.fn(() => Promise.resolve(null)),
    },
    emitters: {
      alignProgressed: {
        listen: (callback: (event: { payload: unknown }) => void) => {
          const listener = (payload: unknown) => {
            callback({ payload });
          };
          listeners.add(listener);
          return Promise.resolve(() => listeners.delete(listener));
        },
      },
    },
    emit: (payload: unknown) => {
      for (const listener of listeners) listener(payload);
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

const { useAlignment } = await import('./useAlignment');

/** The subtitle being watched, as the player hands it over. */
function track(offsetMs = 0, rate = 1): TrackView {
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

/** What the nudge keys have arrived at, and what putting a value back calls. */
function syncOf(offsetMs = 0, rate = 1): Sync {
  return {
    available: true,
    offsetMs,
    rate,
    pending: false,
    nudge: vi.fn(),
    setRate: vi.fn(),
    reset: vi.fn(),
    apply: vi.fn(() => Promise.resolve()),
  };
}

/** An alignment that answers `outcome`, once the test lets it. */
function answering(outcome: AlignmentView) {
  let settle = () => {
    /* replaced below */
  };
  const answered = new Promise<AlignmentView>((resolve) => {
    settle = () => {
      resolve(outcome);
    };
  });
  ipc.alignTrack.mockReturnValue(answered);
  return async () => {
    await act(async () => {
      settle();
      await answered;
    });
  };
}

const ALIGNED: AlignmentView = {
  outcome: 'aligned',
  correction: { offsetMs: 3_500, rate: 1 },
  previous: { offsetMs: -800, rate: 1 },
  confidence: 0.61,
};

function progress(fraction: number): AlignProgressed {
  return { trackId: 3, stage: 'reading', fraction };
}

beforeEach(() => {
  vi.clearAllMocks();
  ipc.cancelAlignment.mockResolvedValue(null);
});

describe('asking for a subtitle to be lined up with its film', () => {
  it('has nothing to measure without a subtitle', () => {
    const { result } = renderHook(() => useAlignment(null, syncOf()));

    act(() => {
      result.current.start();
    });

    expect(result.current.available).toBe(false);
    expect(result.current.state.phase).toBe('idle');
    expect(ipc.alignTrack).not.toHaveBeenCalled();
  });

  it('starts straight away on a file nobody has adjusted', async () => {
    const finish = answering(ALIGNED);
    const sync = syncOf();
    const { result } = renderHook(() => useAlignment(track(), sync));

    act(() => {
      result.current.start();
    });

    expect(ipc.alignTrack).toHaveBeenCalledWith(3);
    expect(result.current.state).toEqual({ phase: 'running', stage: 'reading', fraction: 0 });

    await finish();
    expect(result.current.state).toMatchObject({ phase: 'outcome', outcome: ALIGNED });
  });

  it('follows the reading as it goes', async () => {
    answering(ALIGNED);
    const { result } = renderHook(() => useAlignment(track(), syncOf()));

    act(() => {
      result.current.start();
    });
    await act(async () => {
      // The subscription is asked for asynchronously, so it is in place a tick
      // after the hook that wanted it rendered.
      await Promise.resolve();
      emit(progress(0.4));
    });

    expect(result.current.state).toEqual({ phase: 'running', stage: 'reading', fraction: 0.4 });
  });

  it('ignores how another track is getting on', async () => {
    answering(ALIGNED);
    const { result } = renderHook(() => useAlignment(track(), syncOf()));

    act(() => {
      result.current.start();
    });
    await act(async () => {
      await Promise.resolve();
      emit({ trackId: 9, stage: 'reading', fraction: 0.9 });
    });

    expect(result.current.state).toMatchObject({ fraction: 0 });
  });

  /*
   * A value somebody arrived at themselves is evidence about the file. Nothing
   * records where a correction came from, so anything other than the file as
   * written is treated as theirs and asked about.
   */
  it('asks before replacing a correction already in force', () => {
    answering(ALIGNED);
    const { result } = renderHook(() => useAlignment(track(-1_250), syncOf(-1_250)));

    act(() => {
      result.current.start();
    });

    expect(result.current.state.phase).toBe('confirming');
    expect(ipc.alignTrack).not.toHaveBeenCalled();
  });

  it('asks about a nudge that has not been written down yet', () => {
    answering(ALIGNED);
    // The track still says nought, because the value is a second away from
    // being kept. It is still somebody's own work.
    const { result } = renderHook(() => useAlignment(track(0), syncOf(-400)));

    act(() => {
      result.current.start();
    });

    expect(result.current.state.phase).toBe('confirming');
  });

  it('goes ahead once that is answered', () => {
    answering(ALIGNED);
    const { result } = renderHook(() => useAlignment(track(-1_250), syncOf(-1_250)));

    act(() => {
      result.current.start();
    });
    act(() => {
      result.current.confirm();
    });

    expect(ipc.alignTrack).toHaveBeenCalledWith(3);
    expect(result.current.state.phase).toBe('running');
  });

  it('leaves it alone where the question goes unanswered', () => {
    answering(ALIGNED);
    const { result } = renderHook(() => useAlignment(track(-1_250), syncOf(-1_250)));

    act(() => {
      result.current.start();
    });
    act(() => {
      result.current.dismiss();
    });

    expect(result.current.state.phase).toBe('idle');
    expect(ipc.alignTrack).not.toHaveBeenCalled();
  });
});

describe('what comes of an alignment', () => {
  it('reads the track back once a correction has been written', async () => {
    const finish = answering(ALIGNED);
    const sync = syncOf();
    const { result } = renderHook(() => useAlignment(track(), sync));

    act(() => {
      result.current.start();
    });
    await finish();

    // The row already says this. Writing it again is how the copy the player
    // is reading catches up with what the alignment put there.
    await waitFor(() => {
      expect(sync.apply).toHaveBeenCalledWith(ALIGNED.correction);
    });
  });

  it('writes nothing back for an outcome that changed nothing', async () => {
    const finish = answering({ outcome: 'no-audio' });
    const sync = syncOf();
    const { result } = renderHook(() => useAlignment(track(), sync));

    act(() => {
      result.current.start();
    });
    await finish();

    expect(sync.apply).not.toHaveBeenCalled();
    expect(result.current.state).toMatchObject({ outcome: { outcome: 'no-audio' } });
  });

  it('puts back exactly what it replaced', async () => {
    const finish = answering(ALIGNED);
    const sync = syncOf();
    const { result } = renderHook(() => useAlignment(track(), sync));

    act(() => {
      result.current.start();
    });
    await finish();
    act(() => {
      result.current.undo();
    });

    expect(sync.apply).toHaveBeenLastCalledWith(ALIGNED.previous);
    expect(result.current.state).toMatchObject({ undone: true });
  });

  it('stops when it is asked to, and says the track was left alone', async () => {
    const finish = answering({ outcome: 'stopped' });
    const sync = syncOf();
    const { result } = renderHook(() => useAlignment(track(), sync));

    act(() => {
      result.current.start();
    });
    act(() => {
      result.current.cancel();
    });

    expect(ipc.cancelAlignment).toHaveBeenCalled();

    // The answer still comes back through the call that was already waiting.
    await finish();
    expect(result.current.state).toMatchObject({ outcome: { outcome: 'stopped' } });
    expect(sync.apply).not.toHaveBeenCalled();
  });

  it('says something even when the library itself fails', async () => {
    ipc.alignTrack.mockRejectedValue(new Error('the library is locked'));
    const { result } = renderHook(() => useAlignment(track(), syncOf()));

    await act(async () => {
      result.current.start();
      await Promise.resolve();
    });

    await waitFor(() => {
      expect(result.current.state).toMatchObject({
        outcome: { outcome: 'unreadable', message: 'the library is locked' },
      });
    });
  });

  it('says nothing about a track that is no longer the one being read', async () => {
    const finish = answering(ALIGNED);
    const sync = syncOf();
    const { rerender, result } = renderHook(
      ({ watching }: { watching: TrackView }) => useAlignment(watching, sync),
      { initialProps: { watching: track() } },
    );

    act(() => {
      result.current.start();
    });
    rerender({ watching: { ...track(), id: 8 } });
    await finish();

    expect(result.current.state.phase).toBe('idle');
    expect(sync.apply).not.toHaveBeenCalled();
  });

  it('forgets an outcome when it is dismissed', async () => {
    const finish = answering(ALIGNED);
    const { result } = renderHook(() => useAlignment(track(), syncOf()));

    act(() => {
      result.current.start();
    });
    await finish();
    act(() => {
      result.current.dismiss();
    });

    expect(result.current.state.phase).toBe('idle');
  });
});
