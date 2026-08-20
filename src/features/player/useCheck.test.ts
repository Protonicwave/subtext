import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { AlignmentView, CueView } from '@/shared/ipc/bindings';
import { grounds, landing } from '@/test/alignment';
import { LEAD_MS, busiestMomentOf } from './check';
import type { Alignment, AlignState } from './useAlignment';
import { useCheck } from './useCheck';
import type { Transport } from './usePlayback';

/**
 * The check, as the player owns it.
 *
 * The alignment and the transport are both handed in, so what is left here is
 * the question of when there is anything to check and what answering it does.
 */

const WRITTEN: AlignmentView = {
  outcome: 'aligned',
  correction: { offsetMs: 2_400, rate: 1 },
  previous: { offsetMs: 0, rate: 1 },
  grounds: grounds(0.7),
  landing: landing(0.93),
  asWritten: landing(0.11),
  reference: { against: 'speech' },
};

/** A film with a scene of dialogue in it, and quiet either side. */
const CUES: CueView[] = [0, 60_000, 300_000, 302_000, 304_000, 306_000].map((startMs, at) => ({
  index: at + 1,
  startMs,
  endMs: startMs + 1_500,
  text: 'line',
  position: null,
}));

function alignmentIn(state: AlignState): Alignment {
  return {
    available: true,
    state,
    start: vi.fn(),
    confirm: vi.fn(),
    cancel: vi.fn(),
    undo: vi.fn(),
    dismiss: vi.fn(),
  };
}

function transportOf(): Transport {
  return {
    toggle: vi.fn(),
    play: vi.fn(),
    positionNow: vi.fn(() => 0),
    seekTo: vi.fn(),
    skipBy: vi.fn(),
    setVolume: vi.fn(),
    toggleMute: vi.fn(),
  };
}

function check(state: AlignState, cues: readonly CueView[] = CUES) {
  const alignment = alignmentIn(state);
  const transport = transportOf();
  const held = renderHook(({ align }: { align: Alignment }) => useCheck(align, cues, transport), {
    initialProps: { align: alignment },
  });
  return { held, alignment, transport };
}

/** A measurement that was made and written, which is the case worth checking. */
function written(outcome: AlignmentView = WRITTEN, undone = false): AlignState {
  return { phase: 'outcome', outcome, undone };
}

describe('watching a measurement land', () => {
  it('is offered once a measurement has been written', () => {
    const { held } = check(written());
    expect(held.result.current.offered).toBe(true);
  });

  /*
   * There is nothing to check about a refusal. Nothing was written, the file is
   * as it was, and watching it would show what it always showed.
   */
  it('is not offered where the measurement was refused', () => {
    const { held } = check({ phase: 'outcome', outcome: { outcome: 'no-audio' }, undone: false });
    expect(held.result.current.offered).toBe(false);
  });

  it('is not offered before anything has been measured', () => {
    const { held } = check({ phase: 'idle' });
    expect(held.result.current.offered).toBe(false);
  });

  /** A measurement that has been taken back is not in force to be judged. */
  it('is not offered once the measurement has been put back', () => {
    const { held } = check(written(WRITTEN, true));
    expect(held.result.current.offered).toBe(false);
  });

  /*
   * A track whose lines could not be read has nowhere to send anybody, and an
   * offer that would decline the moment it was taken up is worse than none.
   */
  it('is not offered for a film with no lines to watch for', () => {
    const { held } = check(written(), []);
    expect(held.result.current.offered).toBe(false);
  });

  it('goes to the busiest dialogue in the film and plays it', () => {
    const { held, transport } = check(written());

    act(() => {
      held.result.current.start();
    });

    expect(transport.seekTo).toHaveBeenCalledWith(300_000 - LEAD_MS);
    expect(transport.seekTo).toHaveBeenCalledWith(busiestMomentOf(CUES));
    // Playing rather than toggling, so a film paused to read the sentence is
    // running by the time the stretch arrives.
    expect(transport.play).toHaveBeenCalled();
    expect(held.result.current.watching).toBe(true);
  });

  it('stops asking once the lines have been said to land', () => {
    const { held } = check(written());

    act(() => {
      held.result.current.start();
    });
    act(() => {
      held.result.current.keep();
    });

    expect(held.result.current.watching).toBe(false);
    // Answered, so it is not put again for the same measurement.
    expect(held.result.current.offered).toBe(false);
  });

  /*
   * Putting it back is the undo that was already there rather than a second way
   * of undoing the same thing.
   */
  it('puts the measurement back through the undo that already exists', () => {
    const { held, alignment } = check(written());

    act(() => {
      held.result.current.start();
    });
    act(() => {
      held.result.current.putBack();
    });

    expect(alignment.undo).toHaveBeenCalled();
    expect(held.result.current.watching).toBe(false);
  });

  /*
   * Measuring again is a fresh question, whatever was said about the last
   * answer. Keeping one measurement should not silence the offer for the next.
   */
  it('asks again about a measurement that was made after the last was answered', () => {
    const { held, transport } = check(written());

    act(() => {
      held.result.current.start();
    });
    act(() => {
      held.result.current.keep();
    });
    expect(held.result.current.offered).toBe(false);

    held.rerender({
      align: alignmentIn(written({ ...WRITTEN, correction: { offsetMs: -600, rate: 1 } })),
    });

    expect(held.result.current.offered).toBe(true);
    expect(held.result.current.watching).toBe(false);
    expect(transport.play).toHaveBeenCalledTimes(1);
  });
});
