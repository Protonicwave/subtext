import { describe, expect, it } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { remainingOf } from './remaining';

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  shelf: { name: 'films', path: '/films' },
  title: 'Heat',
  year: 1995,
  durationMs: null,
  posterPath: null,
  accent: null,
  missing: false,
  details: null,
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

/** A film stopped a given number of minutes into a film of a given length. */
function stoppedAt(minutes: number, of: number): FilmView {
  return {
    ...film,
    position: {
      positionMs: minutes * 60_000,
      durationMs: of * 60_000,
      finished: false,
      updatedAt: 0,
      progress: minutes / of,
    },
  };
}

describe('how much of a film is left', () => {
  it('counts in minutes for what is left of an evening', () => {
    expect(remainingOf(stoppedAt(122, 170))).toBe('48 min left');
    expect(remainingOf(stoppedAt(169, 170))).toBe('1 min left');
  });

  it('groups into hours once there are any', () => {
    expect(remainingOf(stoppedAt(50, 170))).toBe('2 hr left');
    expect(remainingOf(stoppedAt(38, 170))).toBe('2 hr 12 min left');
  });

  it('does not offer a film as having no time left', () => {
    expect(remainingOf(stoppedAt(170, 170))).toBe('Nearly finished');
    // A position past the end, which a shorter cut of the same film gives.
    expect(remainingOf(stoppedAt(180, 170))).toBe('Nearly finished');
  });

  it('says something useful when the running time is not known yet', () => {
    expect(remainingOf(film)).toBe('Carry on');
    expect(
      remainingOf({
        ...film,
        position: {
          positionMs: 600_000,
          durationMs: null,
          finished: false,
          updatedAt: 0,
          progress: null,
        },
      }),
    ).toBe('Carry on');
  });

  it('falls back to what the film says when the position does not carry it', () => {
    const known = {
      ...film,
      durationMs: 170 * 60_000,
      position: {
        positionMs: 122 * 60_000,
        durationMs: null,
        finished: false,
        updatedAt: 0,
        progress: null,
      },
    } satisfies FilmView;

    expect(remainingOf(known)).toBe('48 min left');
  });
});
