import type { Id } from '@/shared/ipc/bindings';
import type { DropPoint } from '@/shared/window/drops';
import { settleCover } from './useCover';

/**
 * A picture dropped on a film, rather than films dropped on the window.
 *
 * The window is the drop target for everything, which is what makes adding
 * folders work anywhere. That leaves one question to answer here: whether this
 * particular drop was somebody handing a film its cover, in which case nothing
 * is added to the library and a picture is settled instead.
 *
 * Two things have to be true. It landed on something that says which film it
 * belongs to, which the tile and the film page both do. And it is a picture, by
 * its name, which is the only thing that can be known before the file is read.
 * The name is a claim and the back end holds it to what the file begins with,
 * so a film renamed to a picture's name is refused there rather than here; what
 * the name settles is intent, and dropping a film on a tile is still somebody
 * adding a film.
 */

/** The extensions the walk treats as pictures, which this has to agree with. */
const PICTURES = ['.jpg', '.jpeg', '.png', '.webp'];

/**
 * Settles a dropped picture as a film's cover, and says whether it did.
 *
 * False means the drop was not about a cover and belongs to whoever handles
 * the rest of them, which is how one drop never both adds a folder and changes
 * a picture.
 */
export async function coverDropped(paths: string[], at: DropPoint): Promise<boolean> {
  const picture = paths[0];
  if (paths.length !== 1 || picture === undefined || !looksLikePicture(picture)) return false;

  const film = filmUnder(at);
  if (film === null) return false;

  try {
    await settleCover(film, picture);
  } catch {
    // The picture was refused, which the film page says for itself when the
    // choice was made there. A drop has nowhere of its own to say it, and the
    // library is unchanged either way.
  }
  return true;
}

function looksLikePicture(path: string): boolean {
  const name = path.toLowerCase();
  return PICTURES.some((extension) => name.endsWith(extension));
}

/**
 * The film the drop landed on, where it landed on one.
 *
 * Read off the element under the point rather than held in a store, so that a
 * wall of two thousand tiles costs nothing to have as drop targets: a tile says
 * which film it is in its own markup and is asked only once a drop happens.
 */
function filmUnder(at: DropPoint): Id | null {
  const element = document.elementFromPoint(at.x, at.y);
  const marked = element?.closest('[data-film-id]');
  const id = marked?.getAttribute('data-film-id');
  if (id === undefined || id === null) return null;

  const film = Number(id);
  return Number.isFinite(film) ? film : null;
}
