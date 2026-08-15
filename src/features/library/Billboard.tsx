import type { CSSProperties } from 'react';
import type { FilmView } from '@/shared/ipc/bindings';
import { sourceOf } from '@/shared/media/source';
import { Button } from '@/shared/ui/Button';
import { PlayIcon } from '@/shared/ui/Icon';
import { useFilmAccent, useFilmPalette } from './accent';
import { ComposedCover } from './ComposedCover';
import { useFrames } from './frames';
import { remainingOf } from './remaining';
import { resolutionOf, runtimeOf } from './runtime';
import styles from './Billboard.module.css';

/**
 * One film, shown large above the rows.
 *
 * Netflix's idea at a quarter of the volume: no autoplay, no rotation, no
 * carousel, and one picture doing the work with the type staying out of its
 * way. Two scrims rather than one, the vertical seating the type and the
 * horizontal keeping the right of the picture open, which is what stops it
 * reading as a banner.
 *
 * The window takes the film's colours while it is shown, so the light behind
 * the library is the light of the thing being offered.
 */

interface BillboardProps {
  film: FilmView;
  /** Watch it, from where it was left. */
  onPlay: (film: FilmView) => void;
  /** Read what it is first, which is the film's own page. */
  onOpen: (film: FilmView) => void;
}

export function Billboard({ film, onPlay, onOpen }: BillboardProps) {
  const palette = useFilmPalette(film);
  useFilmAccent(film.accent);

  // Where it was stopped, if a frame has been taken from there yet, and the
  // poster otherwise. Both are pictures of the film itself.
  const frame = useFrames((held) => held.frames[film.id]?.url);
  const still = frame ?? (film.posterPath === null ? null : sourceOf(film.posterPath));
  const carryingOn = film.position !== null;

  return (
    <section
      className={styles.billboard}
      aria-labelledby="billboard-title"
      style={
        {
          '--film-accent': palette.primary,
          '--film-pair': palette.pair,
        } as CSSProperties
      }
    >
      <span className={styles.art} aria-hidden="true">
        {still === null ? (
          <ComposedCover film={film} />
        ) : (
          <img className={styles.still} src={still} alt="" draggable={false} />
        )}
      </span>

      <div className={styles.body}>
        <p className={styles.kicker}>{carryingOn ? 'Carry on watching' : 'From your library'}</p>
        <h1 className={styles.title} id="billboard-title">
          {film.title}
        </h1>

        <p className={styles.meta}>
          {factsOf(film).map((fact, at) => (
            <span key={fact} className={styles.fact}>
              {at > 0 && <i className={styles.dot} aria-hidden="true" />}
              {fact}
            </span>
          ))}
        </p>

        <div className={styles.actions}>
          <Button
            tone="primary"
            onClick={() => {
              onPlay(film);
            }}
          >
            <PlayIcon size={13} />
            {carryingOn ? 'Carry on' : 'Play'}
          </Button>
          <Button
            onClick={() => {
              onOpen(film);
            }}
          >
            Details
          </Button>
        </div>
      </div>
    </section>
  );
}

/**
 * What is said under the title, leaving out whatever is not known.
 *
 * A film nobody has opened has no running time, a file with no year in its name
 * has no year, and a film in a container this application does not parse has no
 * resolution. Each of those is a gap rather than a nought.
 */
function factsOf(film: FilmView): string[] {
  return [
    film.year === null ? null : String(film.year),
    runtimeOf(film.durationMs),
    resolutionOf(film),
    film.position === null ? null : remainingOf(film),
  ].filter((fact) => fact !== null);
}
