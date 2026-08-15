import { beforeEach, describe, expect, it } from 'vitest';
import { breadcrumbFor, useNavigation } from './routes';

const library = { screen: 'library' } as const;

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

  it('opens a film where it was left', () => {
    useNavigation.getState().openFilm(7);

    expect(useNavigation.getState().route).toEqual({ screen: 'player', filmId: 7 });
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
    expect(breadcrumbFor({ screen: 'player', filmId: 7 }, title)).toBe('Heat');
    // A film the library has not read back yet.
    expect(breadcrumbFor({ screen: 'player', filmId: 9 }, title)).toBe('Playing');
  });
});
