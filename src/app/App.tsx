import { useEffect } from 'react';
import { MotionConfig } from 'motion/react';
import { useAppearance } from '@/shared/settings/appearance';
import { useSettings } from '@/shared/settings/useSettings';
import { useChrome } from '@/shared/window/chrome';
import { CloseIcon } from '@/shared/ui/Icon';
import { LibraryScreen } from '@/features/library/LibraryScreen';
import { useLibrary } from '@/features/library/useLibrary';
import { PlayerScreen } from '@/features/player/PlayerScreen';
import { SettingsScreen } from '@/features/settings/SettingsScreen';
import { DropZone } from '@/features/onboarding/DropZone';
import { FirstRun } from '@/features/onboarding/FirstRun';
import { ImportFlow } from '@/features/onboarding/ImportFlow';
import { useImport } from '@/features/onboarding/useImport';
import { ShortcutSheet } from '@/features/settings/ShortcutSheet';
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
  const clearProblem = useLibrary((library) => library.clearProblem);
  const importProblem = useImport((state) => state.problem);
  const dismissImport = useImport((state) => state.dismiss);
  const loadSettings = useSettings((state) => state.load);
  const settingsProblem = useSettings((state) => state.problem);
  const clearSettingsProblem = useSettings((state) => state.clearProblem);
  const motion = useSettings((state) => state.settings.motion);

  useEffect(() => {
    void refresh();
    void loadSettings();
  }, [refresh, loadSettings]);

  useBackdrop();
  useAppearance();
  useEscape(back);

  // Nothing is being watched, so there is nothing to show and only one thing
  // worth doing. Held back until the library has been read, so that the first
  // run screen does not flash up in front of somebody who has a library.
  const firstRun = loaded && folders.length === 0;

  // The first of the three that has something to say, with the way to put it
  // away again. None of these stops a person carrying on, so a strip that could
  // not be dismissed would sit over the window for the rest of the session
  // reporting a failure already read and understood.
  const failure = [
    { says: problem, clear: clearProblem },
    { says: importProblem, clear: dismissImport },
    { says: settingsProblem, clear: clearSettingsProblem },
  ].find((each) => each.says !== null);

  return (
    /*
     * The system's own preference decides whether things move, unless somebody
     * has said here that they would rather nothing did. There is no third
     * answer: a setting that forced animation on over the system's objection
     * would be a setting for overruling somebody who has already spoken.
     */
    <MotionConfig reducedMotion={motion === 'reduced' ? 'always' : 'user'}>
      <TitleBar />

      <main className={styles.stage}>
        {route.screen === 'library' && <LibraryScreen />}
        {route.screen === 'settings' && <SettingsScreen />}
        {route.screen === 'player' && <PlayerScreen filmId={route.filmId} />}
      </main>

      {firstRun && <FirstRun />}

      {failure !== undefined && (
        <p role="alert" className={styles.problem}>
          {failure.says}
          <button
            type="button"
            className={styles.dismiss}
            aria-label="Dismiss"
            onClick={failure.clear}
          >
            <CloseIcon size={11} />
          </button>
        </p>
      )}

      <ShortcutSheet />
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
  const backdrop = useChrome().backdrop;

  useEffect(() => {
    if (backdrop) document.documentElement.dataset.backdrop = 'mica';
  }, [backdrop]);
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
