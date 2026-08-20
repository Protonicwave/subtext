import { act, fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    chooseFolder: vi.fn(() => Promise.resolve(null)),
    rescan: vi.fn(() => Promise.resolve(null)),
    writePreference: vi.fn(() => Promise.resolve(null)),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { Palette } = await import('./Palette');
const { SHOWN } = await import('./matching');
const { useLibrary } = await import('@/features/library/useLibrary');
const { useSheet } = await import('@/features/library/useSheet');
const { useNavigation } = await import('@/app/routes');
const { useSettings } = await import('@/shared/settings/useSettings');
const { DEFAULTS } = await import('@/shared/settings/schema');

function film(id: number, title: string, shelf = 'Films'): FilmView {
  return {
    id,
    folderId: 1,
    path: `/films/${shelf}/${title}.mkv`,
    shelf: { name: shelf, path: `/films/${shelf}` },
    title,
    year: 1995,
    addedAt: 0,
    durationMs: null,
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
  };
}

const LIBRARY = [
  film(1, 'Heat', 'Crime'),
  film(2, 'The Third Man', 'Noir'),
  film(3, 'Amélie', 'French'),
];

/** The palette, up, over a library. */
async function open(films: FilmView[] = LIBRARY) {
  useLibrary.setState({ films, resumable: [], loaded: true });
  render(<Palette />);
  await userEvent.keyboard('{Control>}k{/Control}');
}

/** What the list is showing, in the order it is showing it. */
function rows(): string[] {
  return screen.getAllByRole('option').map((row) => row.textContent);
}

/** Which row the arrows are on. */
function selected(): string {
  const row = screen
    .getAllByRole('option')
    .find((each) => each.getAttribute('aria-selected') === 'true');
  return row?.textContent ?? '';
}

/** The row at the top of the list, which is the one Enter would take. */
function firstRow(): HTMLElement {
  const [row] = screen.getAllByRole('option');
  if (row === undefined) throw new Error('The list is empty');
  return row;
}

/** The row one of the actions is drawn in. */
function rowSaying(name: string): HTMLElement {
  const row = screen.getAllByRole('option').find((each) => each.textContent.includes(name));
  if (row === undefined) throw new Error(`No row says ${name}`);
  return row;
}

describe('the palette', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSheet.setState({ filmId: null });
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
    useSettings.setState({ settings: DEFAULTS });
    useLibrary.setState({ films: [], resumable: [], loaded: true });
  });

  it('is not there until the key is pressed', () => {
    render(<Palette />);

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  /*
   * From every screen, which is the point of it. The key is bound to the window
   * rather than to anything drawn, so what screen is behind it decides nothing.
   */
  it('comes up on the key from wherever somebody is', async () => {
    useNavigation.setState({ route: { screen: 'player', filmId: 1 }, previous: null });
    await open();

    expect(screen.getByRole('dialog', { name: 'Find a film' })).toBeInTheDocument();
  });

  it('goes away again on the same key', async () => {
    await open();
    await userEvent.keyboard('{Control>}k{/Control}');

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  // Nothing typed, so there is nothing to find yet and the palette says what
  // else it is for.
  it('opens on what the application can do', async () => {
    await open();

    expect(screen.getByText('What Subtext can do')).toBeInTheDocument();
    expect(rows()).toEqual([
      expect.stringContaining('Choose a folder to watch'),
      expect.stringContaining('Show the library as covers or as a list'),
      expect.stringContaining('Read the watched folders again'),
      expect.stringContaining('Open settings'),
    ]);
  });

  it('teaches the key beside each thing it can do', async () => {
    await open();

    expect(within(rowSaying('Open settings')).getByText('Ctrl ,')).toBeInTheDocument();
  });

  it('selects the first result the moment it opens', async () => {
    await open();

    expect(selected()).toContain('Choose a folder to watch');
  });

  it('narrows to the films called that, with the films first', async () => {
    await open();
    await userEvent.keyboard('third');

    expect(rows()).toEqual([expect.stringContaining('The Third Man')]);
    expect(screen.getByText('Films')).toBeInTheDocument();
  });

  it('finds a film by the folder it is filed in', async () => {
    await open();
    await userEvent.keyboard('noir');

    expect(rows()).toEqual([expect.stringContaining('The Third Man')]);
  });

  it('shows the year and the folder beside the title', async () => {
    await open();
    await userEvent.keyboard('heat');

    expect(within(firstRow()).getByText('1995')).toBeInTheDocument();
    expect(within(firstRow()).getByText('Crime')).toBeInTheDocument();
  });

  /*
   * Films and actions are drawn as two groups and moved through as one list, so
   * the arrows walk from the last film onto the first action without stopping
   * at the heading between them.
   */
  it('walks from the films into the actions and round again', async () => {
    await open();
    await userEvent.keyboard('the');

    expect(rows()).toEqual([
      expect.stringContaining('The Third Man'),
      expect.stringContaining('Show the library as covers or as a list'),
      expect.stringContaining('Read the watched folders again'),
    ]);

    await userEvent.keyboard('{ArrowDown}');
    expect(selected()).toContain('Show the library as covers or as a list');

    await userEvent.keyboard('{ArrowDown}');
    expect(selected()).toContain('Read the watched folders again');

    await userEvent.keyboard('{ArrowDown}');
    expect(selected()).toContain('The Third Man');

    await userEvent.keyboard('{ArrowUp}');
    expect(selected()).toContain('Read the watched folders again');
  });

  it('opens the film that was chosen, on its page rather than in the player', async () => {
    await open();
    await userEvent.keyboard('amelie{Enter}');

    expect(useSheet.getState().filmId).toBe(3);
    expect(useNavigation.getState().route).toEqual({ screen: 'library' });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('opens the film that was clicked', async () => {
    await open();
    await userEvent.keyboard('heat');
    await userEvent.click(firstRow());

    expect(useSheet.getState().filmId).toBe(1);
  });

  it('says what there was to look through when nothing is called that', async () => {
    await open();
    await userEvent.keyboard('sátántangó');

    expect(screen.getByText(/Nothing in your films is called that/)).toBeInTheDocument();
    expect(screen.getByText(/There are 3 films in the library/)).toBeInTheDocument();
  });

  /*
   * A palette is for finding one film, not for reading a library. What it does
   * not draw is counted and said, since a list that stopped without mentioning
   * it would read as the whole answer.
   */
  it('counts the films it did not draw', async () => {
    await open(Array.from({ length: 60 }, (_, at) => film(at + 1, `Film ${String(at)}`)));
    await userEvent.keyboard('film');

    expect(screen.getByText(/and 20 more/)).toBeInTheDocument();
  });

  it('closes on Escape and leaves the film alone', async () => {
    await open();
    await userEvent.keyboard('heat{Escape}');

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(useSheet.getState().filmId).toBeNull();
  });

  it('puts focus back where it found it', async () => {
    useLibrary.setState({ films: LIBRARY, resumable: [], loaded: true });
    render(
      <>
        <button type="button">A tile</button>
        <Palette />
      </>,
    );

    const tile = screen.getByRole('button', { name: 'A tile' });
    tile.focus();

    await userEvent.keyboard('{Control>}k{/Control}');
    expect(screen.getByRole('combobox')).toHaveFocus();

    await userEvent.keyboard('{Escape}');
    expect(tile).toHaveFocus();
  });

  /*
   * The figure the palette is built around. What a keystroke costs is the
   * folded array walked once and the rows that fit redrawn, and neither grows
   * with a library that is not being drawn: ten thousand films cost the walk
   * and forty rows, the same as forty films cost.
   *
   * The bound is loose against the figure the palette has to meet, because a
   * redraw under jsdom is not a redraw on a screen. What it is here to catch is
   * the palette starting to do work for every film in the library rather than
   * for every film it draws.
   */
  it('redraws over ten thousand films without walking them all', async () => {
    await open(Array.from({ length: 10_000 }, (_, at) => film(at + 1, `Film ${String(at)}`)));
    const field = screen.getByRole('combobox');

    const at = performance.now();
    act(() => {
      fireEvent.change(field, { target: { value: 'film 4' } });
    });
    const took = performance.now() - at;

    expect(screen.getAllByRole('option')).toHaveLength(SHOWN);
    expect(took).toBeLessThan(100);
  });

  it('is emptied between one opening and the next', async () => {
    await open();
    await userEvent.keyboard('heat{Escape}');
    await userEvent.keyboard('{Control>}k{/Control}');

    expect(screen.getByRole('combobox')).toHaveValue('');
  });
});

/*
 * Every action mirrors a control that already exists somewhere, and doing it
 * from here has to be doing the same thing rather than a second version of it.
 */
describe('the actions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
    useSettings.setState({ settings: DEFAULTS });
  });

  it('switches the library between covers and the list', async () => {
    await open();
    await userEvent.keyboard('covers{Enter}');

    expect(useSettings.getState().settings.libraryView).toBe('list');
  });

  it('opens the settings screen', async () => {
    await open();
    await userEvent.keyboard('settings{Enter}');

    expect(useNavigation.getState().route).toEqual({ screen: 'settings' });
  });

  it('asks for a folder', async () => {
    await open();
    await userEvent.keyboard('folder to{Enter}');

    expect(ipc.chooseFolder).toHaveBeenCalled();
  });

  it('reads the watched folders again', async () => {
    await open();
    await userEvent.keyboard('watched folders{Enter}');

    expect(ipc.rescan).toHaveBeenCalled();
  });

  /*
   * The palette teaches the keys rather than replacing them, so a key it lists
   * has to do the same thing without the palette being up at all.
   */
  it('does the same from the key alone', async () => {
    render(<Palette />);

    await userEvent.keyboard('{Control>}l{/Control}');

    expect(useSettings.getState().settings.libraryView).toBe('list');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
