import { useEffect, useRef, type CSSProperties } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { FilmView } from '@/shared/ipc/bindings';
import { PosterTile } from './PosterTile';
import { RAIL_CAPTION, RAIL_GAP, railTileFor, TILE_RATIO } from './grid';
import { CARRYING_ON, type Shelf as ShelfRow } from './shelves';
import { useNearby } from './useNearby';
import { useSetting } from '@/shared/settings/useSettings';
import styles from './Shelf.module.css';

/**
 * One row of the library: a folder somebody made, and what is in it.
 *
 * The rail is virtualised across, for the same reason the wall is virtualised
 * down. A folder of six hundred films is six hundred tiles, and the twelve that
 * fit across the window are the only ones worth existing.
 *
 * The rail runs past the page padding to the edge of the window, so that a row
 * with more in it than fits reads as continuing rather than as stopping.
 */

/** How many tiles either side of the rail are kept ready. */
const OVERSCAN = 4;

interface ShelfProps {
  shelf: ShelfRow;
  onOpen: (film: FilmView) => void;
}

export function Shelf({ shelf, onOpen }: ShelfProps) {
  const scroller = useRef<HTMLDivElement>(null);
  const [section, nearby] = useNearby();

  // A rail is scrolled rather than wrapped, so the size that was chosen is the
  // width of the tile itself here rather than the number that fit across.
  const tile = railTileFor(useSetting('tileSize'));

  // As above the wall: the virtualiser hands back answers that change as the
  // rail is scrolled, and a remembered one would be wrong.
  // eslint-disable-next-line react-hooks/incompatible-library -- see above
  const virtualiser = useVirtualizer({
    horizontal: true,
    count: shelf.films.length,
    getScrollElement: () => scroller.current,
    estimateSize: () => tile + RAIL_GAP,
    overscan: OVERSCAN,
    initialRect: { width: window.innerWidth, height: window.innerHeight },
  });

  // A rail whose tiles changed width has slots in the wrong places until the
  // virtualiser is told to look at them again.
  useEffect(() => {
    virtualiser.measure();
  }, [virtualiser, tile]);

  const heading = `shelf-${shelf.key}`;

  return (
    <section
      className={styles.shelf}
      ref={section}
      aria-labelledby={heading}
      style={
        {
          '--rail-tile': `${String(tile)}px`,
          '--rail-gap': `${String(RAIL_GAP)}px`,
          '--rail-ratio': String(TILE_RATIO),
          '--caption-height': `${String(RAIL_CAPTION)}px`,
        } as CSSProperties
      }
    >
      <div className={styles.head}>
        <h2 className={styles.name} id={heading}>
          {shelf.name}
        </h2>
        {shelf.path !== null && (
          <span className={styles.path} data-selectable>
            {shelf.path}
          </span>
        )}
        <span className={styles.count}>
          {shelf.films.length} {shelf.films.length === 1 ? 'film' : 'films'}
        </span>
      </div>

      <div className={styles.rail} ref={scroller}>
        {/* As wide as every tile it would hold, and a little further, so the
            last one can be scrolled clear of the edge of the window. */}
        <div
          className={styles.track}
          style={{ '--track': `${String(virtualiser.getTotalSize())}px` } as CSSProperties}
        >
          {nearby &&
            virtualiser.getVirtualItems().map((item) => {
              const film = shelf.films[item.index];
              if (!film) return null;

              return (
                <div
                  key={film.id}
                  className={styles.slot}
                  style={{ transform: `translateX(${String(item.start)}px)` }}
                >
                  <PosterTile film={film} onOpen={onOpen} shared={shelf.key !== CARRYING_ON} />
                </div>
              );
            })}
        </div>
      </div>
    </section>
  );
}
