import type { FilmView } from '@/shared/ipc/bindings';
import type { Settings } from '@/shared/settings/schema';

/**
 * Which picture a film is drawn with, wherever it is drawn at the shape of a
 * cover.
 *
 * A film's poster is a cached image, and the row it came from says what it was
 * made of: artwork somebody put on the disk, or a frame taken out of the film
 * because there was no artwork anywhere. The two are not the same claim and are
 * no longer drawn the same way. Artwork is the cover. A frame is a guess an
 * algorithm made at a fifth of the way in, and a wall of guesses reads as a
 * decoder having run rather than as a shelf of films, so a film with no artwork
 * is drawn from its own title instead.
 *
 * Anybody who would rather see the film is one setting away from it, and a
 * frame drawn under that setting keeps its own shape rather than being cut down
 * the middle.
 *
 * Here rather than in the tile because the tile is not the only place a cover
 * is drawn at two by three, and the film page morphs out of the tile: the two
 * showing different pictures for the same film would be visible in the
 * transition between them.
 */

export type Picture =
  /** An image from the disk, drawn as the cover it is. */
  | { kind: 'artwork'; path: string }
  /** A frame from the film, drawn as a frame. */
  | { kind: 'frame'; path: string }
  /** Nothing to draw, so the film is composed from what is known about it. */
  | { kind: 'composed' };

export function pictureFor(film: FilmView, withoutArtwork: Settings['withoutArtwork']): Picture {
  // No poster has been drawn yet, which is every film for the first minute
  // after a folder is added and every film a capture failed on.
  if (film.posterPath === null) return { kind: 'composed' };

  // Any source but none means a person or a tool put that image there, which
  // is the whole reason the source is stored beside the path.
  if (film.coverSource !== 'none') return { kind: 'artwork', path: film.posterPath };

  return withoutArtwork === 'frame'
    ? { kind: 'frame', path: film.posterPath }
    : { kind: 'composed' };
}
