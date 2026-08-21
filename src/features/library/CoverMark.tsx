import { useState, type KeyboardEvent } from 'react';
import type { FilmView } from '@/shared/ipc/bindings';
import { classes } from '@/shared/ui/classes';
import { coverNameOf, isChosen } from './covers';
import { useCover } from './useCover';
import styles from './CoverMark.module.css';

/**
 * Where a tile's picture came from, and the two ways to change it.
 *
 * The same statement the film page makes, in the shortest form that still says
 * it, and the same two actions behind it. A wall of tiles is where somebody
 * notices that a picture is wrong, so the way to settle it is on the tile as
 * well as on the page rather than only on the page.
 *
 * It appears on hover and on focus and is out of the way otherwise, because a
 * mark on every tile all the time would be a wall of labels rather than a wall
 * of films.
 */
export function CoverMark({ film, shown }: { film: FilmView; shown: boolean }) {
  const [open, setOpen] = useState(false);
  const cover = useCover(film);

  const close = () => {
    setOpen(false);
    cover.dismiss();
  };

  // Taken here so that the grid behind does not also read it as leaving the
  // screen, which is what Escape means everywhere else on this page.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'Escape' || !open) return;
    event.preventDefault();
    event.stopPropagation();
    close();
  };

  return (
    <div
      className={classes(styles.mark, (shown || open) && styles.shown)}
      onKeyDown={onKeyDown}
      onBlur={(event) => {
        // Closed when the keyboard leaves it altogether rather than when it
        // moves between the buttons inside it.
        if (!event.currentTarget.contains(event.relatedTarget)) close();
      }}
    >
      <button
        type="button"
        className={styles.label}
        aria-expanded={open}
        onClick={() => {
          setOpen(!open);
        }}
      >
        {coverNameOf(film.coverSource)}
      </button>

      {open && (
        <div className={styles.actions}>
          <button
            type="button"
            className={styles.action}
            disabled={cover.busy}
            onClick={() => {
              cover.choose();
            }}
          >
            Choose an image
          </button>
          {isChosen(film.coverSource) && (
            <button
              type="button"
              className={styles.action}
              disabled={cover.busy}
              onClick={() => {
                cover.clear();
              }}
            >
              Use what was found
            </button>
          )}
          {cover.problem !== null && (
            <p className={styles.problem} role="status">
              {cover.problem}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
