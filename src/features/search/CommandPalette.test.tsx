import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView, SearchView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    searchDialogue: vi.fn(),
    recentSearches: vi.fn(() => Promise.resolve<string[]>([])),
    rememberSearch: vi.fn((query: string) => Promise.resolve([query])),
    forgetSearches: vi.fn(() => Promise.resolve(null)),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { CommandPalette } = await import('./CommandPalette');
const { useSearch } = await import('./useSearch');
const { useLibrary } = await import('@/features/library/useLibrary');
const { useNavigation } = await import('@/app/routes');

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  title: 'Heat',
  year: 1995,
  durationMs: null,
  posterPath: null,
  accent: null,
  missing: false,
  tracks: [
    {
      id: 3,
      path: '/films/Heat.1995.srt',
      language: 'en',
      forced: false,
      hearingImpaired: false,
      origin: 'sidecar' as const,
      streamNumber: 0,
      form: 'text' as const,
      matchKind: 'exact' as const,
      correction: { offsetMs: 0, rate: 1 },
      cueCount: 1_204,
    },
  ],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

const results: SearchView = {
  shown: 3,
  truncated: false,
  ranked: true,
  films: [
    {
      filmId: 7,
      title: 'Heat',
      year: 1995,
      withheld: 2,
      hits: [
        {
          startMs: 92_000,
          snippet: [
            { text: 'the ', matched: false },
            { text: 'action', matched: true },
            { text: ' is the juice', matched: false },
          ],
        },
        { startMs: 415_000, snippet: [{ text: 'a guy told me one time', matched: false }] },
      ],
    },
    {
      filmId: 9,
      title: 'Ronin',
      year: 1998,
      withheld: 0,
      hits: [{ startMs: 12_000, snippet: [{ text: 'the action is elsewhere', matched: false }] }],
    },
  ],
};

/** The palette, with the shortcut it is opened by. */
function palette() {
  render(<CommandPalette />);
  return userEvent.setup();
}

/** The line the arrows are resting on, which is the one Enter opens. */
function highlighted(): string {
  const chosen = document.querySelector('[role="option"][aria-selected="true"]');
  return chosen?.textContent ?? '';
}

describe('the command palette', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    ipc.searchDialogue.mockResolvedValue(results);
    useLibrary.setState({
      folders: [],
      films: [film, { ...film, id: 9, title: 'Ronin', year: 1998 }],
      resumable: [],
      loaded: true,
    });
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
    useSearch.setState({
      open: false,
      query: '',
      film: null,
      scope: null,
      results: null,
      answered: '',
      searching: false,
      problem: null,
      recent: [],
    });
  });

  it('opens on Ctrl+K from anywhere and closes on the same keys', async () => {
    const user = palette();

    await user.keyboard('{Control>}k{/Control}');
    expect(screen.getByRole('combobox')).toBeInTheDocument();

    await user.keyboard('{Control>}k{/Control}');
    await waitFor(() => {
      expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
    });
  });

  it('finds lines and groups them under the films they are said in', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');

    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(screen.getByText('the action is elsewhere')).toBeInTheDocument();
    });
    expect(screen.getByText('Heat')).toBeInTheDocument();
    expect(screen.getByText('Ronin')).toBeInTheDocument();
    // The timecode against each line, which is where it takes you.
    expect(screen.getByText('1:32')).toBeInTheDocument();
    expect(screen.getByText('3 lines in 2 films')).toBeInTheDocument();
  });

  it('says how many more a film had than it is showing', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');
    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(screen.getByText(/2 more in this film/)).toBeInTheDocument();
    });
  });

  it('opens the film at the line, at the moment it is said', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');
    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(highlighted()).toContain('the action is the juice');
    });
    await user.keyboard('{Enter}');

    expect(useNavigation.getState().route).toMatchObject({
      screen: 'player',
      filmId: 7,
      at: { ms: 92_000 },
    });
    // Closed behind itself, so the film is what is on the screen.
    expect(useSearch.getState().open).toBe(false);
  });

  it('walks the arrows across the film boundaries without stopping on them', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');
    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(highlighted()).toContain('the action is the juice');
    });

    await user.keyboard('{ArrowDown}{ArrowDown}');
    expect(highlighted()).toContain('the action is elsewhere');

    // Past the end and round to the first, which is where the key was going.
    await user.keyboard('{ArrowDown}');
    expect(highlighted()).toContain('the action is the juice');
  });

  it('opens the line under the pointer with the keyboard alone', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');
    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(screen.getByText('the action is elsewhere')).toBeInTheDocument();
    });
    await user.keyboard('{ArrowDown}{Enter}');

    expect(useNavigation.getState().route).toMatchObject({ at: { ms: 415_000 } });
  });

  it('writes down a search that opened something', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');
    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(screen.getByText('the action is elsewhere')).toBeInTheDocument();
    });
    await user.keyboard('{Enter}');

    expect(ipc.rememberSearch).toHaveBeenCalledWith('action');
  });

  it('keeps the search to the film being watched, and widens it on Ctrl+Shift+K', async () => {
    useNavigation.setState({ route: { screen: 'player', filmId: 7, at: null }, previous: null });
    const user = palette();

    await user.keyboard('{Control>}k{/Control}');
    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(ipc.searchDialogue).toHaveBeenCalledWith('action', 7);
    });
    expect(screen.getByRole('button', { name: /This film/ })).toBeInTheDocument();

    await user.keyboard('{Control>}{Shift>}K{/Shift}{/Control}');

    await waitFor(() => {
      expect(ipc.searchDialogue).toHaveBeenLastCalledWith('action', null);
    });
  });

  it('says what was searched and how much of it when nothing matched', async () => {
    ipc.searchDialogue.mockResolvedValue({ shown: 0, truncated: false, ranked: true, films: [] });
    const user = palette();

    await user.keyboard('{Control>}k{/Control}');
    await user.type(screen.getByRole('combobox'), 'helicopter');

    await waitFor(() => {
      expect(screen.getByText(/2,408 lines of your 2 films/)).toBeInTheDocument();
    });
    expect(screen.getByText(/“helicopter”/)).toBeInTheDocument();
  });

  it('offers what was searched for before, and searches one when it is chosen', async () => {
    ipc.recentSearches.mockResolvedValue(['the action', 'paris']);
    const user = palette();

    await user.keyboard('{Control>}k{/Control}');

    await waitFor(() => {
      expect(screen.getByText('the action')).toBeInTheDocument();
    });

    await user.keyboard('{Enter}');

    await waitFor(() => {
      expect(ipc.searchDialogue).toHaveBeenCalledWith('the action', null);
    });
  });

  it('sends one search for a word typed at speed rather than one per letter', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');

    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(ipc.searchDialogue).toHaveBeenCalled();
    });
    expect(ipc.searchDialogue).toHaveBeenCalledTimes(1);
    expect(ipc.searchDialogue).toHaveBeenCalledWith('action', null);
  });

  it('clears what it found the moment the field is emptied', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');
    await user.type(screen.getByRole('combobox'), 'action');

    await waitFor(() => {
      expect(screen.getByText('the action is elsewhere')).toBeInTheDocument();
    });

    await user.clear(screen.getByRole('combobox'));

    expect(screen.queryByText('the action is elsewhere')).not.toBeInTheDocument();
  });

  it('closes on Escape without going back a screen as well', async () => {
    const user = palette();
    await user.keyboard('{Control>}k{/Control}');

    await user.keyboard('{Escape}');

    await waitFor(() => {
      expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
    });
  });
});
