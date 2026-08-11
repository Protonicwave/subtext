import { create } from 'zustand';
import type { Id } from '@/shared/ipc/bindings';

/**
 * Where the window is.
 *
 * Three screens and no URLs. A desktop application has no address bar to keep
 * in step and nothing to deep link from, so a router would buy history and
 * path parsing that nothing here needs, and cost a dependency and a layer of
 * indirection over what is a single value.
 *
 * The player carries the film it is playing in the route itself, because a
 * player without one is not a state the application can be in.
 */
export type Route =
  | { readonly screen: 'library' }
  | { readonly screen: 'player'; readonly filmId: Id; readonly at: Moment | null }
  | { readonly screen: 'settings' };

/**
 * A moment a film was asked to open at, rather than resumed at.
 *
 * The count is what tells one jump from the next. Searching for the same line
 * twice names the same millisecond both times, and without something to tell
 * them apart the second one would look like the state the player is already in
 * and move nothing.
 */
export interface Moment {
  readonly ms: number;
  readonly count: number;
}

/** Screens reached by name alone. A film is opened through `openFilm`. */
type Elsewhere = Exclude<Route, { screen: 'player' }>;

interface Navigation {
  readonly route: Route;
  /** Where the last move came from, so Escape can go back to it. */
  readonly previous: Route | null;
  readonly go: (route: Elsewhere) => void;
  /**
   * Opens a film, at a given moment if one is named rather than where it was
   * last left. What a search result does when it is chosen.
   */
  readonly openFilm: (filmId: Id, atMs?: number) => void;
  readonly back: () => void;
}

/** How many films have been opened at a named moment. See [`Moment`]. */
let jumps = 0;

export const useNavigation = create<Navigation>((set) => ({
  route: { screen: 'library' },
  previous: null,
  go: (route) => {
    set((navigation) =>
      // Going where you already are is not a move, and recording it would make
      // Escape a no-op afterwards.
      navigation.route.screen === route.screen ? navigation : { route, previous: navigation.route },
    );
  },
  openFilm: (filmId, atMs) => {
    jumps += 1;
    const at = atMs === undefined ? null : { ms: atMs, count: jumps };

    set((navigation) => {
      const route = { screen: 'player', filmId, at } as const;
      const playing = navigation.route.screen === 'player' && navigation.route.filmId === filmId;

      // Already watching this film: the moment moves, but where Escape goes
      // does not, because nothing new was left behind to come back to.
      if (playing) return at === null ? navigation : { ...navigation, route };
      return { route, previous: navigation.route };
    });
  },
  back: () => {
    set((navigation) => ({
      route: navigation.previous ?? { screen: 'library' },
      previous: null,
    }));
  },
}));

/** What the title bar puts after the app mark for each screen. */
export function breadcrumbFor(route: Route, titleOf: (id: Id) => string | undefined): string {
  switch (route.screen) {
    case 'library':
      return 'Your films';
    case 'settings':
      return 'Settings';
    case 'player':
      return titleOf(route.filmId) ?? 'Playing';
  }
}
