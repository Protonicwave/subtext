import { useEffect } from 'react';
import { MotionConfig } from 'motion/react';
import { ipc } from '@/shared/ipc/client';
import { LibraryScreen } from '@/features/library/LibraryScreen';
import { useLibrary } from '@/features/library/useLibrary';
import { PlayerScreen } from '@/features/player/PlayerScreen';
import { SettingsScreen } from '@/features/settings/SettingsScreen';
import { DropZone } from '@/features/onboarding/DropZone';
import { FirstRun } from '@/features/onboarding/FirstRun';
import { ImportFlow } from '@/features/onboarding/ImportFlow';
import { useImport } from '@/features/onboarding/useImport';
import { TitleBar } from './TitleBar';
import { useNavigation } from './routes';
import styles from './App.module.css';

export function App() {
  const route = useNavigation((navigation) => navigation.route);
  const back = useNavigation((navigation) => navigation.back);
  const refresh = useLibrary((library) => library.refresh);
  const loaded = useLibrary((library) => library.loaded);
  const folders = useLibrary((library) => library.folders);
  const problem = useLibrary((library) => library.problem);
  const importProblem = useImport((state) => state.problem);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useBackdrop();
  useEscape(back);

  // Nothing is being watched, so there is nothing to show and only one thing
  // worth doing. Held back until the library has been read, so that the first
  // run screen does not flash up in front of somebody who has a library.
  const firstRun = loaded && folders.length === 0;

  return (
    // The system's own preference decides whether things move, rather than a
    // setting of ours that would then have to agree with it.
    <MotionConfig reducedMotion="user">
      <TitleBar />

      <main className={styles.stage}>
        {route.screen === 'library' && <LibraryScreen />}
        {route.screen === 'settings' && <SettingsScreen />}
        {route.screen === 'player' && <PlayerScreen filmId={route.filmId} />}
      </main>

      {firstRun && <FirstRun />}

      {(problem ?? importProblem) !== null && (
        <p role="alert" className={styles.problem}>
          {problem ?? importProblem}
        </p>
      )}

      <ImportFlow />
      <DropZone />
    </MotionConfig>
  );
}

/**
 * Asks the window what it turned out to be, and tells the stylesheet.
 *
 * Only Windows 11 puts the desktop behind the window, and the surfaces drawn on
 * top are translucent there and opaque everywhere else. One attribute on the
 * root element is all the difference amounts to.
 */
function useBackdrop() {
  useEffect(() => {
    void ipc
      .windowChrome()
      .then((chrome) => {
        if (chrome.backdrop) document.documentElement.dataset.backdrop = 'mica';
      })
      .catch(() => {
        // A window that will not say how it is drawn is drawn the plain way.
      });
  }, []);
}

/** Escape goes back a screen, from anywhere that is not already the library. */
function useEscape(back: () => void) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      // A sheet that is open handles its own Escape and stops it here.
      if (event.key === 'Escape' && !event.defaultPrevented) back();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [back]);
}
