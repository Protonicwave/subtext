import { createRef } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { DEFAULTS } from '@/shared/settings/schema';
import { useSettings } from '@/shared/settings/useSettings';
import { Spines } from './Spines';

vi.mock('@/shared/ipc/client', () => ({
  ipc: { writePreference: vi.fn(() => Promise.resolve(null)) },
  reasonFor: (failure: unknown) => String(failure),
}));

/*
 * How wide the shelf is, which nothing under a test runner can answer: jsdom
 * lays nothing out, so every element it holds is nought wide and every row
 * would hold one spine. The number is an ordinary window's, so that the rows
 * here are the rows somebody would see.
 */
const WIDTH = 1_400;

vi.mock('@/shared/ui/useWidth', () => ({
  useWidth: () => [() => undefined, WIDTH, null],
}));

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Crime/Heat.1995.mkv',
  shelf: { name: 'Crime', path: '/films/Crime' },
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
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

function make(id: number, changes: Partial<FilmView> = {}): FilmView {
  return { ...film, id, title: `Film ${String(id)}`, ...changes };
}

const open = vi.fn();

function show(films: FilmView[]) {
  const scroller = createRef<HTMLDivElement>();
  render(
    <div ref={scroller} style={{ height: '900px', overflow: 'auto' }}>
      <Spines films={films} scroller={scroller} onOpen={open} />
    </div>,
  );
}

describe('the spines', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettings.setState({ settings: DEFAULTS });
  });

  it('draws nothing at all for a library with nothing in it', () => {
    show([]);

    expect(screen.queryAllByRole('button')).toEqual([]);
  });

  it('names the one film there is', () => {
    show([film]);

    expect(screen.getByRole('button', { name: 'Heat' })).toBeInTheDocument();
  });

  it('opens a film’s page when its spine is chosen', async () => {
    show([film]);

    await userEvent.click(screen.getByRole('button', { name: 'Heat' }));

    expect(open).toHaveBeenCalledWith(film);
  });

  /*
   * The whole point of the view: several hundred films on the screen together,
   * where the wall of the same window holds a couple of dozen.
   */
  it('draws several hundred films at once', () => {
    show(Array.from({ length: 400 }, (_, at) => make(at + 1)));

    expect(screen.getByRole('button', { name: 'Film 1' })).toBeInTheDocument();
    expect(screen.getAllByRole('button').length).toBeGreaterThan(200);
  });

  /*
   * A title longer than its edge is the ordinary case rather than the awkward
   * one. Cutting it is the stylesheet's, which is why what is asserted here is
   * that the whole title is still there to be read out and to be found by: a
   * spine that held a shortened title would be a spine nobody could search for.
   */
  it('keeps the whole of a title too long for its spine', () => {
    const long = 'The Assassination of Jesse James by the Coward Robert Ford';
    show([{ ...film, title: long }]);

    expect(screen.getByRole('button', { name: long })).toHaveTextContent(long);
  });

  /*
   * There is no room on an edge for the badge a tile carries, so a film whose
   * file has gone says so in its name.
   */
  it('says which films are missing their files', () => {
    show([{ ...film, missing: true }]);

    expect(screen.getByRole('button', { name: 'Heat · Missing' })).toBeInTheDocument();
  });

  /*
   * A thousand spines inside one frame, holding no image between them.
   *
   * The two are one property. A spine is a button and a title, and what keeps a
   * large library cheap is that the rows out of view do not exist and that
   * nothing here ever decodes a picture. Both are asserted rather than the
   * timing alone, since a render under a test runner is not a paint and the
   * count is the part that would not change in a browser.
   */
  it('draws a thousand spines quickly and holds no picture', () => {
    const many = Array.from({ length: 1_000 }, (_, at) => make(at + 1));

    const at = performance.now();
    show(many);
    const took = performance.now() - at;

    expect(document.querySelectorAll('img, video, canvas')).toHaveLength(0);

    // Never more than the window holds and a few rows either side, however
    // large the library. This is the number the drawing budget rests on, and
    // unlike the timing it says the same thing outside a test runner.
    const drawn = screen.getAllByRole('button').length;
    expect(drawn).toBeLessThan(400);
    expect(drawn).toBeGreaterThan(200);

    // Loose against the budget, because of what a test runner is. What it
    // catches is a return to drawing every film, which is the only thing here
    // that could cost an order of magnitude.
    expect(took).toBeLessThan(1_000);
  });
});
