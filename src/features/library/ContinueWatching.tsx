import type { CSSProperties } from 'react';
import type { FilmView } from '@/shared/ipc/bindings';
import { sourceOf } from '@/shared/media/source';
import { classes } from '@/shared/ui/classes';
import { useSetting } from '@/shared/settings/useSettings';
import { paletteFor } from './accent';
import { ComposedCover } from './ComposedCover';
import { useFrames } from './frames';
import { remainingOf } from './remaining';
import styles from './ContinueWatching.module.css';

/**
 * The row above the grid: what there is to carry on with.
 *
 * Wide rather than tall, and showing the moment somebody stopped rather than
 * the film's poster, because the question this row answers is not which film it
 * is but where you were in it. The poster is what the grid below is for.
 *
 * These tiles take no part in the transition into the player. The same film can
 * be in this row and in the grid at once, and a shared element has to be one
 * element.
 */

interface ContinueWatchingProps {
  films: FilmView[];
  onOpen: (film: FilmView) => void;
}

export function ContinueWatching({ films, onOpen }: ContinueWatchingProps) {
  const frames = useFrames((held) => held.frames);
  const accent = useSetting('accent');
  if (films.length === 0) return null;

  return (
    <section className={styles.row} aria-labelledby="continue-watching">
      <h2 className={styles.heading} id="continue-watching">
        Carry on watching
      </h2>

      <ul className={styles.cards}>
        {films.map((film) => {
          const palette = paletteFor(film, accent);
          const still =
            frames[film.id]?.url ?? (film.posterPath === null ? null : sourceOf(film.posterPath));

          return (
            <li key={film.id}>
              <button
                type="button"
                className={classes(styles.card, film.missing && styles.gone)}
                style={
                  {
                    '--film-accent': palette.primary,
                    '--film-pair': palette.pair,
                  } as CSSProperties
                }
                onClick={() => {
                  onOpen(film);
                }}
              >
                <span className={styles.shot}>
                  {still === null ? (
                    <ComposedCover film={film} />
                  ) : (
                    <img className={styles.still} src={still} alt="" draggable={false} />
                  )}

                  <span className={styles.sliver} aria-hidden="true">
                    <span style={{ width: `${String(percentOf(film))}%` }} />
                  </span>
                </span>

                <span className={styles.name}>{film.title}</span>
                <span className={styles.left}>
                  {film.missing ? 'File is missing' : remainingOf(film)}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** How far through the film the sliver is drawn to. */
function percentOf(film: FilmView): number {
  return Math.round((film.position?.progress ?? 0) * 100);
}
