import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { CoverSourceView, FilmView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    chooseImage: vi.fn((): Promise<string | null> => Promise.resolve(null)),
    chooseCover: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
    clearCover: vi.fn((): Promise<FilmView> => Promise.reject(new Error('not under test'))),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { CoverReport } = await import('./CoverReport');
const { coverNameOf } = await import('./covers');

const film = {
  id: 1,
  folderId: 1,
  path: '/films/Crime/Heat.1995.mkv',
  shelf: { name: 'Crime', path: '/films/Crime' },
  title: 'Heat',
  year: 1995,
  addedAt: Date.UTC(2026, 7, 12),
  durationMs: 170 * 60_000,
  posterPath: null,
  coverPath: null,
  coverSource: 'none',
  accent: null,
  missing: false,
  details: null,
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

/** A film covered from one of the places, with the picture that covered it. */
function covered(id: number, source: CoverSourceView, coverPath: string | null): FilmView {
  return { ...film, id, title: `Film ${String(id)}`, coverSource: source, coverPath };
}

/** One of a set of elements, which stops the test rather than reading as null. */
function nth(elements: HTMLElement[], at: number): HTMLElement {
  const found = elements[at];
  if (!found) throw new Error(`there is no element ${String(at)}`);
  return found;
}

/** One row of the tally, in the order the places are stated in. */
function place(at: number): HTMLElement {
  return nth(screen.getAllByRole('listitem'), at);
}

describe('the report on where the covers came from', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  /*
   * Every place the back end can settle on has to be able to appear, and each
   * has to say which picture stands behind it, since a tally nobody can check
   * against the disk is a tally nobody has any reason to believe.
   */
  it('names each place, how many films it covered, and a picture from it', () => {
    render(
      <CoverReport
        films={[
          covered(1, 'chosen', '/pictures/heat.jpg'),
          covered(2, 'in-file', '/films/Ronin.1998.mkv'),
          covered(3, 'beside', '/films/Ran.1985.jpg'),
          covered(4, 'beside', '/films/Solaris.1972.jpg'),
          covered(5, 'sidecar', '/films/art/sicario.jpg'),
          covered(6, 'folder-above', '/box/poster.jpg'),
        ]}
      />,
    );

    expect(screen.getAllByRole('listitem')).toHaveLength(5);
    expect(within(place(0)).getByText(coverNameOf('chosen'))).toBeInTheDocument();
    expect(within(place(0)).getByText('/pictures/heat.jpg')).toBeInTheDocument();
    // The two films beside their pictures are counted together, under one of
    // the pictures rather than under both.
    expect(within(place(2)).getByText('2')).toBeInTheDocument();
    expect(within(place(2)).getByText('/films/Ran.1985.jpg')).toBeInTheDocument();
    expect(within(place(4)).getByText(coverNameOf('folder-above'))).toBeInTheDocument();
  });

  it('says nothing about a place that covered no films', () => {
    render(<CoverReport films={[covered(1, 'beside', '/films/Ran.1985.jpg')]} />);

    expect(screen.getByText(coverNameOf('beside'))).toBeInTheDocument();
    expect(screen.queryByText(coverNameOf('sidecar'))).not.toBeInTheDocument();
    expect(screen.queryByText(coverNameOf('chosen'))).not.toBeInTheDocument();
  });

  /*
   * The numbers have to add up to what the library holds, so a film with no
   * artwork is stated rather than quietly missing from the tally.
   */
  it('lists every film that is left over, with the way to settle it', async () => {
    render(<CoverReport films={[film, { ...film, id: 2, title: 'Ronin' }]} />);

    expect(screen.getByText(/2 films have no artwork/)).toBeInTheDocument();
    const settle = screen.getAllByRole('button', { name: 'Choose a picture' });
    expect(settle).toHaveLength(2);

    await userEvent.click(nth(settle, 0));
    expect(ipc.chooseImage).toHaveBeenCalledTimes(1);
  });

  it('says nothing about leftovers when every film has a cover', () => {
    render(<CoverReport films={[covered(1, 'beside', '/films/Ran.1985.jpg')]} />);

    expect(screen.queryByText(/no artwork/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
