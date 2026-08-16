import { useEffect, useMemo, type RefObject } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { FilmView } from '@/shared/ipc/bindings';
import { useWidth } from '@/shared/ui/useWidth';
import { useSetting } from '@/shared/settings/useSettings';
import { PosterTile } from './PosterTile';
import { CAPTION, columnsFor, rowHeight, rowsOf, TILE_GAP, wallTileFor } from './grid';
import { useOffsetTop } from './useOffsetTop';
import styles from './Wall.module.css';

/**
 * Every film at once, in one grid.
 *
 * Virtualised by row: a library of ten thousand films is ten thousand tiles,
 * and ten thousand tiles is thirty thousand elements, which no amount of care
 * elsewhere would make scroll smoothly. Only the rows in view and a few either
 * side exist at any moment.
 *
 * The screen owns the scrolling and hands it down, because the virtualiser has
 * to be told which element is scrolled to know what is in view.
 */

/** How many rows either side of the view are kept ready. */
const OVERSCAN = 3;

interface WallProps {
  films: FilmView[];
  scroller: RefObject<HTMLDivElement | null>;
  onOpen: (film: FilmView) => void;
}

export function Wall({ films, scroller, onOpen }: WallProps) {
  const [grid, width, node] = useWidth();
  const above = useOffsetTop(scroller, node);

  // The tiles are as wide as the width divided between them, so the size that
  // was chosen decides how many go in a row rather than how wide each one is
  // drawn. A window is a fixed thing to fill either way.
  const least = wallTileFor(useSetting('tileSize'));
  const columns = columnsFor(width, least);
  const rows = useMemo(() => rowsOf(films, columns), [films, columns]);
  const size = rowHeight(width, columns, TILE_GAP, CAPTION);

  // The compiler will not memoise a component holding this, because the
  // virtualiser hands back functions whose answers change as the view moves and
  // a remembered one would be wrong. That is the arrangement being asked for
  // here: this component redraws as it is scrolled, and it is nine elements.
  // eslint-disable-next-line react-hooks/incompatible-library -- see above
  const virtualiser = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scroller.current,
    estimateSize: () => size,
    overscan: OVERSCAN,
    // What is above the wall in the same scroller, so that a scroll position is
    // read as a position within the wall rather than within the page.
    scrollMargin: above,
    // What to assume until the element has been measured. The window is the
    // closest thing to the truth available before a layout has happened, and it
    // is the difference between a first paint with tiles in it and one without.
    initialRect: { width: window.innerWidth, height: window.innerHeight },
  });

  // A window that changed width has rows of a different height, and one that
  // changed height has moved the wall down the page. The virtualiser holds the
  // old numbers until it is told to look again.
  useEffect(() => {
    virtualiser.measure();
  }, [virtualiser, size, above]);

  return (
    <div
      className={styles.grid}
      ref={grid}
      style={{ height: `${String(virtualiser.getTotalSize())}px` }}
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
          {(rows[row.index] ?? []).map((film) => (
            <PosterTile key={film.id} film={film} onOpen={onOpen} />
          ))}
        </div>
      ))}
    </div>
  );
}
