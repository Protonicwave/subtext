import { describe, expect, it } from 'vitest';
import type { SearchView } from '@/shared/ipc/bindings';
import { foundSaying, nothingSaying } from './saying';

function found(changes: Partial<SearchView> = {}): SearchView {
  return {
    shown: 8,
    truncated: false,
    ranked: true,
    films: [
      { filmId: 7, title: 'Heat', year: 1995, withheld: 0, hits: [] },
      { filmId: 9, title: 'Ronin', year: 1998, withheld: 0, hits: [] },
    ],
    ...changes,
  };
}

describe('saying what was found', () => {
  it('counts the lines and the films they are in', () => {
    expect(foundSaying(found())).toBe('8 lines in 2 films');
  });

  it('leaves the count of films off when they are all in one', () => {
    const alone = found().films.slice(0, 1);

    expect(foundSaying(found({ shown: 1, films: alone }))).toBe('1 line');
  });

  it('says when it is showing the first of them rather than all of them', () => {
    expect(foundSaying(found({ shown: 100, truncated: true }))).toBe(
      'the first 100 lines in 2 films',
    );
  });

  it('says when these are the first matches rather than the best ones', () => {
    // Without this, a search whose best match is further down the list looks
    // like a search that ranked badly.
    expect(foundSaying(found({ ranked: false }))).toBe(
      '8 lines in 2 films, in the order they are said',
    );
  });
});

describe('saying why nothing was found', () => {
  const library = { films: 48, lines: 312_004 };

  it('says how much was searched, not just that there was nothing', () => {
    expect(nothingSaying('helicopter', null, library)).toBe(
      'Nothing in the 312,004 lines of your 48 films matches “helicopter”.',
    );
  });

  it('names the film when the search was kept to one, and offers the way out', () => {
    const said = nothingSaying('helicopter', { title: 'Heat', lines: 1_204 }, library);

    expect(said).toContain('the 1,204 lines of Heat');
    expect(said).toContain('Widening the search');
  });

  it('says a film has no subtitles rather than that nobody in it says the word', () => {
    expect(nothingSaying('paris', { title: 'Heat', lines: 0 }, library)).toContain(
      'no subtitle file paired with it',
    );
  });

  it('tells a library with no dialogue indexed what search actually reads', () => {
    expect(nothingSaying('paris', null, { films: 48, lines: 0 })).toContain(
      'None of your films has a subtitle file',
    );
  });

  it('tells an empty library to add a folder', () => {
    expect(nothingSaying('paris', null, { films: 0, lines: 0 })).toContain('no films to search');
  });
});
