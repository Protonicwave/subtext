import { describe, expect, it } from 'vitest';
import type { FilmView, PositionView } from '@/shared/ipc/bindings';
import { isFinished, startAtOf } from './resume';

function film(position: PositionView | null): FilmView {
  return {
    id: 1,
    folderId: 1,
    path: '/films/Heat.1995.mkv',
    title: 'Heat',
    year: 1995,
    durationMs: 10_260_000,
    posterPath: null,
    accent: null,
    missing: false,
    details: null,
    tracks: [],
    chosenTrackId: null,
    subtitlesOff: false,
    position,
  };
}

function at(positionMs: number, finished = false): PositionView {
  return { positionMs, durationMs: 10_260_000, finished, updatedAt: 0, progress: null };
}

describe('where a film opens', () => {
  it('starts at the beginning when it has never been watched', () => {
    expect(startAtOf(film(null), 5_000)).toBe(0);
  });

  it('runs up to where it was left', () => {
    expect(startAtOf(film(at(600_000)), 5_000)).toBe(595_000);
  });

  it('never runs up past the start of the film', () => {
    expect(startAtOf(film(at(2_000)), 5_000)).toBe(0);
  });

  it('starts a finished film again from the beginning', () => {
    expect(startAtOf(film(at(10_200_000, true)), 5_000)).toBe(0);
  });
});

describe('when a film counts as watched', () => {
  it('does not wait for the very end', () => {
    expect(isFinished(9_952_200, 10_260_000, 0.97)).toBe(true);
    expect(isFinished(10_260_000, 10_260_000, 0.97)).toBe(true);
  });

  it('is not finished in the middle', () => {
    expect(isFinished(600_000, 10_260_000, 0.97)).toBe(false);
  });

  it('is not finished while the running time is unknown', () => {
    expect(isFinished(600_000, null, 0.97)).toBe(false);
    expect(isFinished(0, 0, 0.97)).toBe(false);
  });
});
