import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    chooseCover: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { coverDropped } = await import('./coverDrops');
const { useLibrary } = await import('./useLibrary');

const film = {
  id: 7,
  folderId: 1,
  path: '/films/Crime/Heat.1995.mkv',
  shelf: { name: 'Crime', path: '/films/Crime' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: 170 * 60_000,
  posterPath: null,
  coverSource: 'none',
  accent: null,
  missing: false,
  details: null,
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

const AT = { x: 40, y: 60 };

/**
 * What the drop landed on.
 *
 * The point comes from the webview and the element from the document, and
 * nothing in a test environment lays either out. The lookup is answered
 * directly rather than by building a page with a tile at those coordinates,
 * and it is defined rather than replaced because the environment has no
 * layout to answer it with in the first place.
 */
function landsOn(element: Element | null) {
  Object.defineProperty(document, 'elementFromPoint', {
    configurable: true,
    value: () => element,
  });
}

function tile(id: number): Element {
  const marked = document.createElement('div');
  marked.setAttribute('data-film-id', String(id));
  const inside = document.createElement('img');
  marked.append(inside);
  return inside;
}

describe('a picture dropped on a film', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useLibrary.setState({ films: [film], resumable: [], loaded: true });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(document, 'elementFromPoint');
  });

  it("becomes that film's cover, and says the drop was handled", async () => {
    ipc.chooseCover.mockReturnValue(Promise.resolve({ ...film, coverSource: 'chosen' as const }));
    landsOn(tile(film.id));

    const settled = await coverDropped(['/pictures/Heat.png'], AT);

    expect(settled).toBe(true);
    expect(ipc.chooseCover).toHaveBeenCalledWith(film.id, '/pictures/Heat.png');
    expect(useLibrary.getState().films[0]?.coverSource).toBe('chosen');
  });

  /*
   * The tile is found from anywhere inside it, since what is under the pointer
   * is whichever element happens to be on top at that moment.
   */
  it('is found from whatever part of the tile it landed on', async () => {
    ipc.chooseCover.mockReturnValue(Promise.resolve(film));
    landsOn(tile(film.id));

    expect(await coverDropped(['/pictures/Heat.jpg'], AT)).toBe(true);
  });

  it('leaves a drop that landed on no film to whoever adds folders', async () => {
    landsOn(document.createElement('div'));

    expect(await coverDropped(['/pictures/Heat.png'], AT)).toBe(false);
    expect(ipc.chooseCover).not.toHaveBeenCalled();
  });

  /*
   * Dropping a film on a tile is still somebody adding a film. The name is what
   * settles the intent; whether the file really is a picture is the back end's
   * question and is asked of the bytes.
   */
  it('leaves anything that is not named as a picture alone', async () => {
    landsOn(tile(film.id));

    expect(await coverDropped(['/films/Ronin.1998.mkv'], AT)).toBe(false);
    expect(ipc.chooseCover).not.toHaveBeenCalled();
  });

  it('leaves a handful of files at once alone', async () => {
    landsOn(tile(film.id));

    const settled = await coverDropped(['/pictures/Heat.png', '/pictures/Ronin.png'], AT);

    expect(settled).toBe(false);
    expect(ipc.chooseCover).not.toHaveBeenCalled();
  });

  it('leaves a drop of nothing alone', async () => {
    landsOn(tile(film.id));

    expect(await coverDropped([], AT)).toBe(false);
  });

  /*
   * A refused picture still counts as handled. The drop was about a cover, and
   * falling through to adding the folder the picture sits in would put a folder
   * of artwork in the library.
   */
  it('does not add a folder when the picture was refused', async () => {
    ipc.chooseCover.mockImplementation(() => Promise.reject(new Error('that is not a picture')));
    landsOn(tile(film.id));

    expect(await coverDropped(['/pictures/Heat.png'], AT)).toBe(true);
  });
});
