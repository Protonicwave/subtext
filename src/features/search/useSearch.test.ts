import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { SearchView } from '@/shared/ipc/bindings';

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

const { useSearch } = await import('./useSearch');

/** Results carrying a marker, so a test can tell one answer from another. */
function answer(shown: number): SearchView {
  return {
    shown,
    truncated: false,
    ranked: true,
    films: [{ filmId: 7, title: 'Heat', year: 1995, withheld: 0, hits: [] }],
  };
}

/** A search whose answer is handed back when the test says so. */
function pending(results: SearchView) {
  let land = () => {
    // Replaced below, before anything can call it.
  };
  const promise = new Promise<SearchView>((resolve) => {
    land = () => {
      resolve(results);
    };
  });

  return { promise, land };
}

describe('searching', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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

  it('keeps the search to the film being watched when it opens over one', () => {
    useSearch.getState().show(7);

    expect(useSearch.getState().scope).toBe(7);
    expect(useSearch.getState().film).toBe(7);
  });

  it('widens the search to the library and puts it back again', () => {
    useSearch.getState().show(7);

    useSearch.getState().toggleScope();
    expect(useSearch.getState().scope).toBeNull();

    useSearch.getState().toggleScope();
    expect(useSearch.getState().scope).toBe(7);
  });

  it('has nothing to widen when no film is playing', () => {
    useSearch.getState().show(null);
    useSearch.getState().toggleScope();

    expect(useSearch.getState().scope).toBeNull();
  });

  it('never lets a slow answer land on top of a faster one that came after it', async () => {
    const slow = pending(answer(1));
    const quick = pending(answer(2));
    ipc.searchDialogue.mockReturnValueOnce(slow.promise).mockReturnValueOnce(quick.promise);

    useSearch.getState().show(null);
    useSearch.getState().type('par');
    const first = useSearch.getState().run();

    useSearch.getState().type('paris');
    const second = useSearch.getState().run();

    // The finished word comes back first, and the prefix behind it lands after.
    quick.land();
    await second;
    slow.land();
    await first;

    expect(useSearch.getState().results?.shown).toBe(2);
    expect(useSearch.getState().answered).toBe('paris');
  });

  it('drops an answer to a search that was typed past', async () => {
    const late = pending(answer(1));
    ipc.searchDialogue.mockReturnValueOnce(late.promise);

    useSearch.getState().show(null);
    useSearch.getState().type('par');
    const search = useSearch.getState().run();

    // Emptied while it was in flight, which is what clearing the field does.
    useSearch.getState().type('');
    late.land();
    await search;

    expect(useSearch.getState().results).toBeNull();
  });

  it('drops an answer to a search whose palette has closed', async () => {
    const late = pending(answer(1));
    ipc.searchDialogue.mockReturnValueOnce(late.promise);

    useSearch.getState().show(null);
    useSearch.getState().type('paris');
    const search = useSearch.getState().run();

    useSearch.getState().hide();
    late.land();
    await search;

    expect(useSearch.getState().results).toBeNull();
    expect(useSearch.getState().open).toBe(false);
  });

  it('does not ask for anything when there is nothing in the field', async () => {
    useSearch.getState().show(null);
    useSearch.getState().type('   ');
    await useSearch.getState().run();

    expect(ipc.searchDialogue).not.toHaveBeenCalled();
  });

  it('searches the film it is kept to, and the library once it is widened', async () => {
    ipc.searchDialogue.mockResolvedValue(answer(1));

    useSearch.getState().show(7);
    useSearch.getState().type('paris');
    await useSearch.getState().run();

    expect(ipc.searchDialogue).toHaveBeenLastCalledWith('paris', 7);

    useSearch.getState().toggleScope();
    await useSearch.getState().run();

    expect(ipc.searchDialogue).toHaveBeenLastCalledWith('paris', null);
  });

  it('shows what went wrong rather than an empty list', async () => {
    ipc.searchDialogue.mockRejectedValue(new Error('the database is locked'));

    useSearch.getState().show(null);
    useSearch.getState().type('paris');
    await useSearch.getState().run();

    expect(useSearch.getState().problem).toBe('the database is locked');
    expect(useSearch.getState().searching).toBe(false);
  });

  it('writes down a search that led somewhere', async () => {
    useSearch.getState().show(null);
    useSearch.getState().type('  paris  ');
    await useSearch.getState().chose();

    expect(ipc.rememberSearch).toHaveBeenCalledWith('paris');
    expect(useSearch.getState().recent).toEqual(['paris']);
  });

  it('opens on an empty field rather than on the last thing searched for', () => {
    useSearch.getState().show(null);
    useSearch.getState().type('paris');
    useSearch.getState().hide();

    useSearch.getState().show(null);

    expect(useSearch.getState().query).toBe('');
    expect(useSearch.getState().results).toBeNull();
  });
});
