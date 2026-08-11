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

interface Navigation {
  readonly route: Route;
  /** Where the last move came from, so Escape can go back to it. */
  readonly previous: Route | null;
  readonly go: (route: Route) => void;
  readonly back: () => void;
}

export const useNavigation = create<Navigation>((set) => ({
  route: { screen: 'library' },
  previous: null,
  go: (route) => {
    set((navigation) =>
      // Going where you already are is not a move, and recording it would make
      // Escape a no-op afterwards.
      navigation.route.screen === route.screen && sameFilm(navigation.route, route)
        ? navigation
        : { route, previous: navigation.route },
    );
  },
  back: () => {
    set((navigation) => ({
      route: navigation.previous ?? { screen: 'library' },
      previous: null,
    }));
  },
}));

function sameFilm(one: Route, other: Route): boolean {
  if (one.screen !== 'player' || other.screen !== 'player') return true;
  return one.filmId === other.filmId;
}

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
