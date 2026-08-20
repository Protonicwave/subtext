import { useEffect, useMemo, type CSSProperties, type RefObject } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { FilmView } from '@/shared/ipc/bindings';
import { useSetting } from '@/shared/settings/useSettings';
import { classes } from '@/shared/ui/classes';
import { useWidth } from '@/shared/ui/useWidth';
import { paletteFor } from './accent';
import { columnsFor, rowsOf, SPINE_GAP, SPINE_HEIGHT, SPINE_WIDTH } from './grid';
import { useOffsetTop } from './useOffsetTop';
import styles from './Spines.module.css';

/**
 * The library as edges, which is how the same discs sat on a shelf.
 *
 * Covers say what each film is and a list says what every film is. Neither
 * shows a few hundred at once, and a library somebody assembled is a thing
 * worth seeing the whole of. A spine costs a title and a colour pair, both of
 * which every film already has, so a thousand of them draw inside one frame and
 * hold no image at all.
 *
 * Rows are virtualised down the screen's own scrolling, the same way the wall
 * is. There is no second code path for a small library: the virtualiser draws
 * every row when every row is in view, so the count decides for itself.
 */

/** How many rows either side of the view are kept ready. */
const OVERSCAN = 3;

interface SpinesProps {
  films: FilmView[];
  scroller: RefObject<HTMLDivElement | null>;
  onOpen: (film: FilmView) => void;
}

export function Spines({ films, scroller, onOpen }: SpinesProps) {
  const [shelf, width, node] = useWidth();
  const above = useOffsetTop(scroller, node);

  // Read once here rather than through the hook on each spine, since there may
  // be a thousand of them and the answer is the same for all of them.
  const accent = useSetting('accent');

  const columns = columnsFor(width, SPINE_WIDTH, SPINE_GAP);
  const rows = useMemo(() => rowsOf(films, columns), [films, columns]);
  const size = SPINE_HEIGHT + SPINE_GAP;

  // As above the wall: the virtualiser hands back answers that change as the
  // page is scrolled, and a remembered one would be wrong.
  // eslint-disable-next-line react-hooks/incompatible-library -- see above
  const virtualiser = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scroller.current,
    estimateSize: () => size,
    overscan: OVERSCAN,
    // What is above the shelf in the same scroller, so that a scroll position
    // is read as a position within it rather than within the page.
    scrollMargin: above,
    initialRect: { width: window.innerWidth, height: window.innerHeight },
  });

  // A window that changed height has moved the shelf down the page, and the
  // virtualiser holds the old number until it is told to look again.
  useEffect(() => {
    virtualiser.measure();
  }, [virtualiser, above]);

  return (
    <div
      className={styles.shelf}
      ref={shelf}
      style={
        {
          '--spine-gap': `${String(SPINE_GAP)}px`,
          '--spine-height': `${String(SPINE_HEIGHT)}px`,
          height: `${String(virtualiser.getTotalSize())}px`,
        } as CSSProperties
      }
    >
      {virtualiser.getVirtualItems().map((row) => (
        <div
          key={row.key}
          className={styles.line}
          style={{
            height: `${String(row.size)}px`,
            transform: `translateY(${String(row.start - above)}px)`,
            gridTemplateColumns: `repeat(${String(columns)}, minmax(0, 1fr))`,
          }}
        >
          {(rows[row.index] ?? []).map((film) => {
            const palette = paletteFor(film, accent);

            return (
              <button
                key={film.id}
                type="button"
                className={classes(styles.spine, film.missing && styles.gone)}
                /* There is no room on an edge for the badge a tile carries, so
                   a film whose file has gone says so in its name and is drawn
                   faint. Nothing else about it changes: it is still a film the
                   library knows about, on a drive that is not plugged in. */
                aria-label={film.missing ? `${film.title} · Missing` : undefined}
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
                <span className={styles.title}>{film.title}</span>
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}
