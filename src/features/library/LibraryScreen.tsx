import { useEffect, useMemo, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { FilmView } from '@/shared/ipc/bindings';
import { FolderIcon } from '@/shared/ui/Icon';
import { Button } from '@/shared/ui/Button';
import { useWidth } from '@/shared/ui/useWidth';
import { useNavigation } from '@/app/routes';
import { useImport } from '@/features/onboarding/useImport';
import { ContinueWatching } from './ContinueWatching';
import { PosterTile } from './PosterTile';
import { columnsFor, rowHeight, rowsOf, TILE_GAP } from './grid';
import { useCapture } from './useCapture';
import { linesOf, useLibrary } from './useLibrary';
import styles from './LibraryScreen.module.css';

/**
 * The screen you land on.
 *
 * The grid is virtualised by row: a library of ten thousand films is ten
 * thousand tiles, and ten thousand tiles is thirty thousand elements, which no
 * amount of care elsewhere would make scroll smoothly. Only the rows in view
 * and a few either side exist at any moment.
 *
 * This screen owns its scrolling rather than leaving it to the shell, because
 * the virtualiser has to be the thing that is scrolled to know what is in view.
 */

/** Room under each tile for the title and the line beneath it. */
const CAPTION = 44;

/** How many rows either side of the view are kept ready. */
const OVERSCAN = 3;

export function LibraryScreen() {
  const films = useLibrary((library) => library.films);
  const folders = useLibrary((library) => library.folders);
  const resumable = useLibrary((library) => library.resumable);
  const chooseFolder = useImport((state) => state.chooseFolder);
  const go = useNavigation((navigation) => navigation.go);

  const scroller = useRef<HTMLDivElement>(null);
  const [grid, width] = useWidth();

  const columns = columnsFor(width);
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
    // What to assume until the element has been measured. The window is the
    // closest thing to the truth available before a layout has happened, and it
    // is the difference between a first paint with tiles in it and one without.
    initialRect: { width: window.innerWidth, height: window.innerHeight },
  });

  // A window that changed width has rows of a different height, and the
  // virtualiser holds the old ones until it is told to look again.
  useEffect(() => {
    virtualiser.measure();
  }, [virtualiser, size]);

  // Nothing is captured until the library has been read, so that a first run
  // does not open a decoder before there is anything to point it at.
  useCapture(films.length > 0);

  const open = (film: FilmView) => {
    go({ screen: 'player', filmId: film.id });
  };

  const lines = films.reduce((total, film) => total + linesOf(film), 0);

  return (
    <div className={styles.screen} ref={scroller}>
      <div className={styles.inner}>
        <header className={styles.top}>
          <h1 className={styles.title}>
            Your films
            <small>
              {films.length === 0
                ? 'Nothing yet'
                : `${count(films.length, 'film')} · ${lines.toLocaleString('en-GB')} lines of dialogue`}
            </small>
          </h1>
          <Button tone="primary" onClick={() => void chooseFolder()}>
            <FolderIcon size={14} />
            Choose folder
          </Button>
        </header>

        <ContinueWatching films={resumable} onOpen={open} />

        {films.length === 0 ? (
          <p className={styles.empty}>
            {folders.length === 0
              ? 'No folders are being watched yet.'
              : 'Nothing was found in the folders being watched.'}
          </p>
        ) : (
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
                  transform: `translateY(${String(row.start)}px)`,
                  gridTemplateColumns: `repeat(${String(columns)}, minmax(0, 1fr))`,
                }}
              >
                {(rows[row.index] ?? []).map((film) => (
                  <PosterTile key={film.id} film={film} onOpen={open} />
                ))}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function count(many: number, thing: string): string {
  return `${String(many)} ${thing}${many === 1 ? '' : 's'}`;
}
