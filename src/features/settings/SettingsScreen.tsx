import { type ReactNode, useState } from 'react';
import { classes } from '@/shared/ui/classes';
import { AppearanceSection } from './AppearanceSection';
import { LibrarySection } from './LibrarySection';
import { LibraryViewSection } from './LibraryViewSection';
import { PlaybackSection } from './PlaybackSection';
import { ShortcutTable } from './ShortcutTable';
import styles from './SettingsScreen.module.css';

/**
 * Everything the application has an opinion about that is worth asking about.
 *
 * A rail of five rather than one long page, because the five have nothing to do
 * with each other: somebody changing how a subtitle looks is not on their way
 * to the folder list. Which one is showing is this component's own state and
 * nothing else's, since it is not worth keeping between visits and there is no
 * address bar to put it in.
 *
 * Five sections and not everything that could be one. A screen that offered a
 * number for every interval the player works to would be longer without being
 * more useful: almost nobody has a view about how long a control bar waits
 * before it hides, and the ones who do are better served by a value chosen
 * carefully than by being asked. What remains is what somebody would go looking
 * for, and what nothing else could answer for them.
 *
 * Every control writes through to the library file as it is used, so there is
 * nothing here to save and nothing to undo. That is why there is no footer.
 */

interface Panel {
  name: string;
  says: string;
  content: () => ReactNode;
}

const PANELS: readonly Panel[] = [
  {
    name: 'Library',
    says: 'Where your films are read from, and what is made of what is in them.',
    content: () => <LibrarySection />,
  },
  {
    name: 'Library view',
    says: 'How the films you have are laid out on the screen you land on.',
    content: () => <LibraryViewSection />,
  },
  {
    name: 'Playback',
    says: 'How a film behaves once it is open, and how its subtitles read.',
    content: () => <PlaybackSection />,
  },
  {
    name: 'Appearance',
    says: 'How much of the film is let into the window around it.',
    content: () => <AppearanceSection />,
  },
  {
    name: 'Shortcuts',
    says: 'Every key the application answers to.',
    content: () => <ShortcutTable />,
  },
];

export function SettingsScreen() {
  const [showing, setShowing] = useState(PANELS[0]?.name ?? '');
  const panel = PANELS.find((known) => known.name === showing) ?? PANELS[0];

  return (
    <div className={styles.screen}>
      <nav className={styles.rail} aria-label="Settings">
        {PANELS.map((known) => (
          <button
            key={known.name}
            type="button"
            className={classes(styles.tab, known.name === panel?.name && styles.here)}
            aria-current={known.name === panel?.name ? 'page' : undefined}
            onClick={() => {
              setShowing(known.name);
            }}
          >
            {known.name}
          </button>
        ))}
      </nav>

      {panel !== undefined && (
        <section className={styles.panel} aria-label={panel.name}>
          <header className={styles.top}>
            <h1 className={styles.title}>{panel.name}</h1>
            <p className={styles.says}>{panel.says}</p>
          </header>

          {panel.content()}
        </section>
      )}
    </div>
  );
}
