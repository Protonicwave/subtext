import { create } from 'zustand';
import type { Id, SearchView } from '@/shared/ipc/bindings';
import { ipc, reasonFor } from '@/shared/ipc/client';

/**
 * What the palette is looking for and what it found.
 *
 * Two things here are worth stating rather than reading out of the code.
 *
 * The first is that a search is only sent once the field has been still for a
 * moment. Typing at speed would otherwise send a query per keystroke, and the
 * ones in the middle are prefixes nobody meant to search for.
 *
 * The second is that answers are checked against the search that is current
 * when they arrive. A query is a round trip, and a short one sent second can
 * come back before a long one sent first. Without the check, typing quickly
 * would leave the results of a half typed word on the screen underneath the
 * word that was finished.
 */

/** How long the field is left alone before the search is sent. */
export const SETTLE_MS = 90;

/** Every search sent, counted. An answer that is not the latest is dropped. */
let issued = 0;

interface SearchState {
  open: boolean;
  query: string;
  /** The film that was playing when the palette opened, if one was. */
  film: Id | null;
  /** The film the search is kept to, or null for the whole library. */
  scope: Id | null;
  results: SearchView | null;
  /** The query the results on screen answer, so the empty state can quote it. */
  answered: string;
  /** A search is on its way. The results already shown stay until it lands. */
  searching: boolean;
  problem: string | null;
  recent: readonly string[];

  /** Opens the palette, kept to the film being watched where there is one. */
  show: (film: Id | null) => void;
  hide: () => void;
  type: (query: string) => void;
  /** Widens the search to the whole library, or puts it back to the film. */
  toggleScope: () => void;
  /** Sends what is in the field. Called once the field has settled. */
  run: () => Promise<void>;
  /** Keeps a search that led somewhere. */
  chose: () => Promise<void>;
  forget: () => Promise<void>;
}

const NOTHING = {
  results: null,
  answered: '',
  searching: false,
  problem: null,
} as const;

export const useSearch = create<SearchState>((set, get) => ({
  open: false,
  query: '',
  film: null,
  scope: null,
  recent: [],
  ...NOTHING,

  show: (film) => {
    // A fresh field every time. Coming back to the palette to look for
    // something else and finding the last search in it, selected, is a worse
    // guess than an empty field.
    set({ open: true, query: '', film, scope: film, ...NOTHING });

    void ipc
      .recentSearches()
      .then((recent) => {
        set({ recent });
      })
      .catch(() => {
        // Not worth a message. The palette works without the list of what was
        // searched for before, and the field is where the attention is anyway.
      });
  },

  hide: () => {
    // The results are dropped with it: they answer a query that is no longer
    // in the field, and holding them would show them for an instant the next
    // time the palette opened.
    issued += 1;
    set({ open: false, query: '', ...NOTHING });
  },

  type: (query) => {
    set({ query });
    // Emptying the field empties the results with it, now rather than after
    // the field settles, so that clearing it never leaves stale lines up.
    if (query.trim() === '') {
      issued += 1;
      set(NOTHING);
    }
  },

  toggleScope: () => {
    const { film, scope } = get();
    if (film === null) return;

    set({ scope: scope === null ? film : null });
  },

  run: async () => {
    const { query, scope } = get();
    if (query.trim() === '') return;

    const ticket = (issued += 1);
    set({ searching: true });

    try {
      const results = await ipc.searchDialogue(query, scope);
      // Something was typed, or the palette closed, while this was in flight.
      if (ticket !== issued) return;

      set({ results, answered: query, searching: false, problem: null });
    } catch (failure) {
      if (ticket !== issued) return;

      set({ ...NOTHING, answered: query, problem: reasonFor(failure) });
    }
  },

  chose: async () => {
    const query = get().query.trim();
    if (query === '') return;

    try {
      const recent = await ipc.rememberSearch(query);
      set({ recent });
    } catch {
      // The search still opened the film, which is what was asked for. Not
      // being able to write it down is not worth interrupting that.
    }
  },

  forget: async () => {
    try {
      await ipc.forgetSearches();
      set({ recent: [] });
    } catch (failure) {
      set({ problem: reasonFor(failure) });
    }
  },
}));
