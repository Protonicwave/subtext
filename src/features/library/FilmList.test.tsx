import { createRef } from 'react';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';
import { DEFAULTS } from '@/shared/settings/schema';
import { useSettings } from '@/shared/settings/useSettings';
import { FilmList } from './FilmList';

vi.mock('@/shared/ipc/client', () => ({
  ipc: { writePreference: vi.fn(() => Promise.resolve(null)) },
  reasonFor: (failure: unknown) => String(failure),
}));

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Crime/Heat.1995.mkv',
  shelf: { name: 'Crime', path: '/films/Crime' },
  title: 'Heat',
  year: 1995,
  addedAt: new Date(2026, 7, 12).getTime(),
  durationMs: 170 * 60_000,
  posterPath: null,
  coverPath: null,
  coverSource: 'none',
  accent: null,
  missing: false,
  details: {
    container: 'Matroska',
    sizeBytes: 15_800_000_000,
    averageBitrate: 12_400_000,
    video: { codec: 'H.264', width: 1920, height: 1080, bitDepth: 8, frameRate: 23.976 },
    audio: [],
  },
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

function make(id: number, changes: Partial<FilmView> = {}): FilmView {
  return { ...film, id, ...changes };
}

const open = vi.fn();

function show(films: FilmView[]) {
  const scroller = createRef<HTMLDivElement>();
  render(
    <div ref={scroller} style={{ height: '900px', overflow: 'auto' }}>
      <FilmList films={films} scroller={scroller} onOpen={open} />
    </div>,
  );
}

/** One row of the table, the header being the first of them. */
function row(at: number): HTMLElement {
  const found = screen.getAllByRole('row')[at];
  if (!found) throw new Error(`there is no row ${String(at)}`);
  return found;
}

/** The titles of the rows, in the order they are drawn. */
function rows(): string[] {
  return screen
    .getAllByRole('row')
    .slice(1)
    .map((line) => within(line).getAllByRole('cell')[0]?.textContent ?? '');
}

describe('the library as a list', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettings.setState({ settings: DEFAULTS, problem: null });
  });

  it('shows every column of what a film is', () => {
    show([film]);

    const cells = within(row(1)).getAllByRole('cell');

    expect(cells.map((cell) => cell.textContent)).toEqual([
      'Heat',
      'Crime',
      '1995',
      '2 hr 50 min',
      'H.264 · 1080p',
      '15.8 GB',
      '12 Aug',
      'Not started',
    ]);
  });

  /*
   * An MP4 is not opened by this application, and a film recorded before the
   * build that reads what a file is has not been looked at either. Neither is a
   * film of no size in a picture of no height.
   */
  it('leaves a fact the film does not carry empty', () => {
    show([make(1, { year: null, durationMs: null, details: null })]);

    const cells = within(row(1)).getAllByRole('cell');

    expect(cells.map((cell) => cell.textContent)).toEqual([
      'Heat',
      'Crime',
      '',
      '',
      '',
      '',
      '12 Aug',
      'Not started',
    ]);
  });

  it('says where somebody got to, and what happened to a film that has gone', () => {
    show([
      make(1, {
        title: 'Ronin',
        position: {
          positionMs: 60_000,
          durationMs: 120_000,
          finished: false,
          updatedAt: 0,
          progress: 0.5,
        },
      }),
      make(2, { title: 'Stalker', missing: true }),
    ]);

    expect(screen.getByRole('img', { name: '50% watched' })).toBeInTheDocument();
    expect(screen.getByText('Missing')).toBeInTheDocument();
  });

  it('opens a film from its title, and from anywhere else along the row', async () => {
    show([film]);

    await userEvent.click(screen.getByRole('button', { name: 'Heat' }));
    expect(open).toHaveBeenCalledTimes(1);

    await userEvent.click(row(1));
    expect(open).toHaveBeenCalledTimes(2);
    expect(open).toHaveBeenLastCalledWith(film);
  });

  it('starts in the order the library arrived in, which is by title', () => {
    show([make(1, { title: 'Stalker' }), make(2, { title: 'Heat' })]);

    expect(rows()).toEqual(['Heat', 'Stalker']);
    expect(screen.getByRole('columnheader', { name: /title/i })).toHaveAttribute(
      'aria-sort',
      'ascending',
    );
  });

  it('sorts by a column when its heading is pressed, and reverses on the next press', async () => {
    show([
      make(1, { title: 'Stalker', year: 1979 }),
      make(2, { title: 'Heat', year: 1995 }),
      make(3, { title: 'Persona', year: 1966 }),
    ]);

    await userEvent.click(screen.getByRole('button', { name: /year/i }));
    expect(rows()).toEqual(['Persona', 'Stalker', 'Heat']);
    expect(screen.getByRole('columnheader', { name: /year/i })).toHaveAttribute(
      'aria-sort',
      'ascending',
    );

    await userEvent.click(screen.getByRole('button', { name: /year/i }));
    expect(rows()).toEqual(['Heat', 'Stalker', 'Persona']);
    expect(screen.getByRole('columnheader', { name: /year/i })).toHaveAttribute(
      'aria-sort',
      'descending',
    );
  });

  /*
   * The sort is a setting rather than something the screen holds, which is what
   * makes it survive going to a film and coming back, and a restart after that.
   */
  it('remembers the column and the direction', async () => {
    show([film]);

    await userEvent.click(screen.getByRole('button', { name: /added/i }));
    await userEvent.click(screen.getByRole('button', { name: /added/i }));

    expect(useSettings.getState().settings.librarySort).toBe('added');
    expect(useSettings.getState().settings.libraryDescending).toBe(true);
  });

  it('starts a column it has just been given the right way up', async () => {
    useSettings.setState({
      settings: { ...DEFAULTS, librarySort: 'year', libraryDescending: true },
    });
    show([film]);

    await userEvent.click(screen.getByRole('button', { name: /size/i }));

    expect(useSettings.getState().settings.librarySort).toBe('size');
    expect(useSettings.getState().settings.libraryDescending).toBe(false);
  });

  /*
   * The property the whole screen rests on: a row costs the same whether there
   * are ten rows behind it or ten thousand.
   */
  it('draws a windowful of rows for a library of ten thousand films', () => {
    const many = Array.from({ length: 10_000 }, (_, at) =>
      make(at + 1, { title: `Film ${String(at + 1).padStart(5, '0')}` }),
    );

    const at = performance.now();
    show(many);
    const took = performance.now() - at;

    expect(screen.getAllByRole('row').length).toBeLessThan(40);
    expect(screen.getByRole('button', { name: 'Film 00001' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Film 09000' })).not.toBeInTheDocument();
    expect(screen.getByRole('table')).toHaveAttribute('aria-rowcount', '10001');

    // Loose enough not to fail on a busy machine, and slow enough to catch a
    // return to drawing all of them.
    expect(took).toBeLessThan(2_000);
  });
});
