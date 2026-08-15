import { useEffect, useState } from 'react';
import { CloseIcon, MaximiseIcon, MinimiseIcon, RestoreIcon, SettingsIcon } from '@/shared/ui/Icon';
import { classes } from '@/shared/ui/classes';
import { windowControls } from '@/shared/window/controls';
import { breadcrumbFor, useNavigation } from './routes';
import { useLibrary } from '@/features/library/useLibrary';
import styles from './TitleBar.module.css';

/**
 * The bar along the top, which the window no longer draws for us.
 *
 * It is the drag handle for the whole window, so everything in it that can be
 * clicked has to opt out of that, which is what the no-drag class does.
 */
export function TitleBar() {
  const route = useNavigation((navigation) => navigation.route);
  const go = useNavigation((navigation) => navigation.go);
  const films = useLibrary((library) => library.films);
  const maximised = useMaximised();

  const breadcrumb = breadcrumbFor(
    route,
    (id) => films.find((film) => film.id === id)?.title ?? undefined,
  );

  return (
    <header className={styles.bar} data-tauri-drag-region>
      <div className={styles.brand} data-tauri-drag-region>
        <span className={styles.mark} />
        Subtext
      </div>

      <p className={styles.crumb} data-tauri-drag-region>
        {breadcrumb}
      </p>

      <div className={styles.spacer} data-tauri-drag-region />

      <button
        type="button"
        className={styles.icon}
        aria-label="Settings"
        aria-current={route.screen === 'settings' ? 'page' : undefined}
        onClick={() => {
          go(route.screen === 'settings' ? { screen: 'library' } : { screen: 'settings' });
        }}
      >
        <SettingsIcon size={13} />
      </button>

      <div className={styles.controls}>
        <button
          type="button"
          className={styles.control}
          aria-label="Minimise"
          onClick={() => void windowControls.minimise()}
        >
          <MinimiseIcon size={11} />
        </button>
        <button
          type="button"
          className={styles.control}
          aria-label={maximised ? 'Restore' : 'Maximise'}
          onClick={() => void windowControls.toggleMaximise()}
        >
          {maximised ? <RestoreIcon size={11} /> : <MaximiseIcon size={11} />}
        </button>
        <button
          type="button"
          className={classes(styles.control, styles.close)}
          aria-label="Close"
          onClick={() => void windowControls.close()}
        >
          <CloseIcon size={11} />
        </button>
      </div>
    </header>
  );
}

/**
 * Whether the window is maximised, kept in step with the window itself.
 *
 * Watched rather than remembered, because the button in this bar is not the
 * only thing that maximises a window: so do a double click on the bar, the
 * snap gestures, and the keyboard shortcuts the platform owns.
 */
function useMaximised(): boolean {
  const [maximised, setMaximised] = useState(false);

  useEffect(() => {
    let watching = true;
    const check = () => {
      void windowControls.isMaximised().then((is) => {
        if (watching) setMaximised(is);
      });
    };
    check();

    let stop: (() => void) | undefined;
    void windowControls.onResized(check).then((unsubscribe) => {
      if (watching) stop = unsubscribe;
      else unsubscribe();
    });

    return () => {
      watching = false;
      stop?.();
    };
  }, []);

  return maximised;
}
