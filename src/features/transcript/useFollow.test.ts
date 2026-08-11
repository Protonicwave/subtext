import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { NONE } from '@/shared/media/cues';
import { useFollow } from './useFollow';

function follow(active = 0) {
  const goTo = vi.fn();
  const view = renderHook(({ line }) => useFollow(line, goTo), {
    initialProps: { line: active },
  });

  return { goTo, view };
}

describe('following the film through the transcript', () => {
  it('puts each line in view as it is spoken', () => {
    const { goTo, view } = follow();
    expect(goTo).toHaveBeenLastCalledWith(0);

    view.rerender({ line: 4 });
    expect(goTo).toHaveBeenLastCalledWith(4);
  });

  it('stays where it is during a silence', () => {
    const { goTo, view } = follow();

    view.rerender({ line: NONE });

    expect(goTo).toHaveBeenCalledTimes(1);
  });

  it('lets go the moment somebody scrolls it', () => {
    const { goTo, view } = follow();
    act(() => {
      view.result.current.release();
    });

    view.rerender({ line: 9 });

    expect(view.result.current.following).toBe(false);
    expect(goTo).not.toHaveBeenCalledWith(9);
  });

  it('never takes itself back', () => {
    // The line being read stays where it was put, however long the film runs
    // on. Nothing here is on a timer, and this is the test that says so.
    const { goTo, view } = follow();
    act(() => {
      view.result.current.release();
    });

    for (const line of [1, 2, 3, 4, 5]) view.rerender({ line });

    expect(goTo).toHaveBeenCalledTimes(1);
  });

  it('catches up with the film when it is asked to', () => {
    const { goTo, view } = follow();
    act(() => {
      view.result.current.release();
    });
    view.rerender({ line: 12 });

    act(() => {
      view.result.current.resume();
    });

    expect(view.result.current.following).toBe(true);
    expect(goTo).toHaveBeenLastCalledWith(12);
  });
});
