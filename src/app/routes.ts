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
  | { readonly screen: 'player'; readonly filmId: Id }
  | { readonly screen: 'settings' };

/** Screens reached by name alone. A film is opened through `openFilm`. */
type Elsewhere = Exclude<Route, { screen: 'player' }>;

interface Navigation {
  readonly route: Route;
  /** Where the last move came from, so Escape can go back to it. */
  readonly previous: Route | null;
  readonly go: (route: Elsewhere) => void;
  /** Opens a film, where it was last left. */
  readonly openFilm: (filmId: Id) => void;
  readonly back: () => void;
}

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
  openFilm: (filmId) => {
    set((navigation) => {
      const playing = navigation.route.screen === 'player' && navigation.route.filmId === filmId;
      // Already watching this film, so nothing moves and nothing new is left
      // behind to come back to.
      if (playing) return navigation;
      return { route: { screen: 'player', filmId }, previous: navigation.route };
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
