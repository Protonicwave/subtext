import { beforeEach, describe, expect, it } from 'vitest';
import { breadcrumbFor, useNavigation } from './routes';

const library = { screen: 'library' } as const;

/** The moment the player was last sent to, if it was sent to one. */
function momentOf() {
  const route = useNavigation.getState().route;
  return route.screen === 'player' ? route.at : null;
}

describe('navigation', () => {
  beforeEach(() => {
    useNavigation.setState({ route: library, previous: null });
  });

  it('remembers where a move came from', () => {
    useNavigation.getState().go({ screen: 'settings' });

    expect(useNavigation.getState().route).toEqual({ screen: 'settings' });

    useNavigation.getState().back();

    expect(useNavigation.getState().route).toEqual(library);
  });

  it('treats going where you already are as no move at all', () => {
    useNavigation.getState().openFilm(7);
    useNavigation.getState().openFilm(7);

    // Recording it would have made going back land on the player again.
    useNavigation.getState().back();

    expect(useNavigation.getState().route).toEqual(library);
  });

  it('counts another film as somewhere else', () => {
    useNavigation.getState().openFilm(7);
    useNavigation.getState().openFilm(8);

    useNavigation.getState().back();

    expect(useNavigation.getState().route).toMatchObject({ screen: 'player', filmId: 7 });
  });

  it('opens a film at the moment a search result named', () => {
    useNavigation.getState().openFilm(7, 92_000);

    expect(useNavigation.getState().route).toMatchObject({
      screen: 'player',
      filmId: 7,
      at: { ms: 92_000 },
    });
  });

  it('tells one jump to the same line from the next', () => {
    useNavigation.getState().openFilm(7, 92_000);
    const first = momentOf();

    useNavigation.getState().openFilm(7, 92_000);
    const again = momentOf();

    // The same millisecond both times. Without something to tell them apart the
    // player would read the second as the state it is already in and stay put.
    expect(again?.ms).toBe(first?.ms);
    expect(again?.count).not.toBe(first?.count);
  });

  it('stays where it came from when a second line of the same film is chosen', () => {
    useNavigation.getState().openFilm(7, 92_000);
    useNavigation.getState().openFilm(7, 415_000);

    // Two search results in one film is one move away from the library, not
    // two, so Escape still leads out rather than back to the first line.
    useNavigation.getState().back();

    expect(useNavigation.getState().route).toEqual(library);
  });

  it('leaves a film opened without a moment to resume where it was left', () => {
    useNavigation.getState().openFilm(7);

    expect(useNavigation.getState().route).toEqual({ screen: 'player', filmId: 7, at: null });
  });

  it('goes back to the library when it came from nowhere', () => {
    useNavigation.setState({ route: { screen: 'settings' }, previous: null });

    useNavigation.getState().back();

    expect(useNavigation.getState().route).toEqual(library);
  });

  it('names the film in the breadcrumb while one is playing', () => {
    const title = (id: number) => (id === 7 ? 'Heat' : undefined);

    expect(breadcrumbFor(library, title)).toBe('Your films');
    expect(breadcrumbFor({ screen: 'settings' }, title)).toBe('Settings');
    expect(breadcrumbFor({ screen: 'player', filmId: 7, at: null }, title)).toBe('Heat');
    // A film the library has not read back yet.
    expect(breadcrumbFor({ screen: 'player', filmId: 9, at: null }, title)).toBe('Playing');
  });
});
