import { useEffect, useState } from 'react';
import { DropIcon } from '@/shared/ui/Icon';
import { onFilesDropped } from '@/shared/window/drops';
import { useImport } from './useImport';
import styles from './DropZone.module.css';

/**
 * Anywhere in the window is somewhere to drop films.
 *
 * There is no target to aim at, because the whole window is the target. This
 * draws the overlay that says so while something is being dragged, and does
 * nothing at all the rest of the time.
 */
export function DropZone() {
  const [over, setOver] = useState(false);
  const addDropped = useImport((state) => state.addDropped);

  useEffect(() => {
    let watching = true;
    let stop: (() => void) | undefined;

    void onFilesDropped({
      over: () => {
        setOver(true);
      },
      left: () => {
        setOver(false);
      },
      drop: (paths) => {
        setOver(false);
        void addDropped(paths);
      },
    }).then((unsubscribe) => {
      if (watching) stop = unsubscribe;
      else unsubscribe();
    });

    return () => {
      watching = false;
      stop?.();
    };
  }, [addDropped]);

  if (!over) return null;

  return (
    <div className={styles.overlay} aria-hidden="true">
      <div className={styles.target}>
        <span className={styles.glyph}>
          <DropIcon size={26} />
        </span>
        <p className={styles.heading}>Drop them here</p>
        <p className={styles.detail}>
          Films and subtitle files together. Subtext works out which belongs to which.
        </p>
      </div>
    </div>
  );
}
