import { SHORTCUTS } from './shortcuts';
import styles from './ShortcutTable.module.css';

/**
 * The keys, as a list of what each one does.
 *
 * Drawn once and shown in two places: the settings screen, where somebody is
 * looking for it, and the card the question mark brings up, where somebody has
 * asked from wherever they happened to be.
 */
export function ShortcutTable() {
  return (
    <div className={styles.groups}>
      {SHORTCUTS.map((group) => (
        <section key={group.title} className={styles.group}>
          <h3 className={styles.title}>{group.title}</h3>
          <dl className={styles.list}>
            {group.shortcuts.map((shortcut) => (
              <div key={`${group.title}:${shortcut.keys.join('+')}`} className={styles.row}>
                <dt className={styles.keys}>
                  {shortcut.keys.map((key) => (
                    <kbd key={key}>{key}</kbd>
                  ))}
                </dt>
                <dd className={styles.does}>{shortcut.does}</dd>
              </div>
            ))}
          </dl>
        </section>
      ))}
    </div>
  );
}
