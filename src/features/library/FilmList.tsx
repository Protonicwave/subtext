import {
  useEffect,
  useMemo,
  useState,
  type CSSProperties,
  type ReactNode,
  type RefObject,
} from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { FilmView } from '@/shared/ipc/bindings';
import { useSetting, useSettings } from '@/shared/settings/useSettings';
import { CaretIcon } from '@/shared/ui/Icon';
import { classes } from '@/shared/ui/classes';
import { paletteFor } from './accent';
import { COLUMNS, ORDER, sortedBy, type SortName } from './columns';
import { dateOf } from './dates';
import { resolutionOf, runtimeOf } from './runtime';
import { sizeOf } from './size';
import { useOffsetTop } from './useOffsetTop';
import styles from './FilmList.module.css';

/**
 * The library as a table, for a library the wall has stopped serving.
 *
 * Virtualised down the same scroller the wall uses, for the same reason: ten
 * thousand rows of eight cells is eighty thousand elements, and only the twenty
 * or so in view are worth existing. The header stays put while they pass under
 * it, which is what makes a long table readable at all.
 *
 * Sorting happens here rather than in the database. The library is already in
 * memory, so a round trip would be slower than the comparison, and the sort has
 * to survive a rescan changing rows nobody asked about.
 */

/** How tall a row is. Fixed, so the virtualiser needs to measure nothing. */
const ROW = 64;

/** How many rows either side of the view are kept ready. */
const OVERSCAN = 8;

interface FilmListProps {
  films: FilmView[];
  scroller: RefObject<HTMLDivElement | null>;
  onOpen: (film: FilmView) => void;
}

export function FilmList({ films, scroller, onOpen }: FilmListProps) {
  const [node, setNode] = useState<HTMLDivElement | null>(null);
  const above = useOffsetTop(scroller, node);

  const sort = useSetting('librarySort');
  const descending = useSetting('libraryDescending');
  const change = useSettings((state) => state.change);
  const accent = useSetting('accent');

  const ordered = useMemo(() => sortedBy(films, sort, descending), [films, sort, descending]);

  // As above the wall: the virtualiser hands back answers that change as the
  // page is scrolled, and a remembered one would be wrong.
  // eslint-disable-next-line react-hooks/incompatible-library -- see above
  const virtualiser = useVirtualizer({
    count: ordered.length,
    getScrollElement: () => scroller.current,
    estimateSize: () => ROW,
    overscan: OVERSCAN,
    scrollMargin: above,
    initialRect: { width: window.innerWidth, height: window.innerHeight },
  });

  // The table starts lower down the page once the window has been resized, and
  // the virtualiser holds the old number until it is told to look again.
  useEffect(() => {
    virtualiser.measure();
  }, [virtualiser, above]);

  /*
   * The first press on a column sorts by it, and the next reverses it. One rule
   * and no column that is special: a table where some headings start one way
   * round and some the other is a table nobody can predict.
   */
  const sortBy = (name: SortName) => {
    if (name === sort) {
      change('libraryDescending', !descending);
      return;
    }
    change('librarySort', name);
    change('libraryDescending', false);
  };

  return (
    <div className={styles.list} role="table" aria-rowcount={ordered.length + 1}>
      <div className={styles.head} role="row">
        {ORDER.map((name) => (
          <div
            key={name}
            role="columnheader"
            aria-sort={name === sort ? (descending ? 'descending' : 'ascending') : 'none'}
            className={classes(styles.heading, COLUMNS[name].figures && styles.figures)}
          >
            <button
              type="button"
              className={classes(styles.sorter, name === sort && styles.sorted)}
              onClick={() => {
                sortBy(name);
              }}
            >
              {COLUMNS[name].name}
              {name === sort && (
                <CaretIcon size={11} className={classes(!descending && styles.up)} />
              )}
            </button>
          </div>
        ))}
      </div>

      <div
        className={styles.rows}
        role="rowgroup"
        ref={setNode}
        style={{ height: `${String(virtualiser.getTotalSize())}px` }}
      >
        {virtualiser.getVirtualItems().map((row) => {
          const film = ordered[row.index];
          if (!film) return null;

          const palette = paletteFor(film, accent);

          return (
            <div
              key={film.id}
              role="row"
              aria-rowindex={row.index + 2}
              className={classes(styles.row, film.missing && styles.gone)}
              style={
                {
                  height: `${String(row.size)}px`,
                  transform: `translateY(${String(row.start - above)}px)`,
                  '--film-accent': palette.primary,
                  '--film-pair': palette.pair,
                } as CSSProperties
              }
              onClick={() => {
                onOpen(film);
              }}
            >
              {ORDER.map((name) => (
                <div
                  key={name}
                  role="cell"
                  className={classes(styles.cell, COLUMNS[name].figures && styles.figures)}
                >
                  {cellOf(name, film, onOpen)}
                </div>
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * What one cell holds.
 *
 * A fact the film does not carry is left empty rather than filled with a zero
 * or a dash: an empty cell in a column of figures reads as not known, which is
 * what it is.
 *
 * The title carries the only control in the row, so that the list is walkable
 * from the keyboard without the row itself becoming a button around eight other
 * things. A press anywhere else on the row does the same thing for a pointer.
 */
function cellOf(name: SortName, film: FilmView, onOpen: (film: FilmView) => void): ReactNode {
  switch (name) {
    case 'title':
      return (
        <>
          <span className={styles.chip} aria-hidden="true" />
          <button
            type="button"
            className={styles.title}
            onClick={(event) => {
              // The row is listening as well, and one press should open one
              // film once.
              event.stopPropagation();
              onOpen(film);
            }}
          >
            {film.title}
          </button>
        </>
      );
    case 'folder':
      return <span className={styles.folder}>{film.shelf.name}</span>;
    case 'year':
      return film.year === null ? '' : String(film.year);
    case 'runtime':
      return runtimeOf(film.durationMs) ?? '';
    case 'video':
      return videoOf(film);
    case 'size':
      return film.details === null ? '' : sizeOf(film.details.sizeBytes);
    case 'added':
      return dateOf(film.addedAt);
    case 'watched':
      return watchedOf(film);
  }
}

/** The picture, as the container described it. */
function videoOf(film: FilmView): string {
  const video = film.details?.video;
  if (!video) return '';

  return [video.codec, resolutionOf(film), video.bitDepth === 10 ? '10-bit' : null]
    .filter((part) => part !== null && part !== '')
    .join(' · ');
}

/** How far through a film somebody is, or what happened to it. */
function watchedOf(film: FilmView): ReactNode {
  if (film.missing) return <span className={styles.missing}>Missing</span>;

  const position = film.position;
  if (position === null) return <span className={styles.faint}>Not started</span>;
  if (position.finished) return 'Watched';
  if (position.progress === null) return <span className={styles.faint}>Started</span>;

  const percent = Math.round(position.progress * 100);
  return (
    <span className={styles.progress} role="img" aria-label={`${String(percent)}% watched`}>
      <span style={{ width: `${String(percent)}%` }} />
    </span>
  );
}
