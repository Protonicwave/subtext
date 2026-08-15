import type { FilmView } from '@/shared/ipc/bindings';
import { factsOf } from './runtime';
import styles from './ComposedCover.module.css';

/**
 * What a film is drawn as when there is no picture for it anywhere.
 *
 * Every film gets a cover. A file the webview will not decode, a drive that was
 * unplugged before the poster was drawn, a film with no artwork inside it and
 * nothing beside it: none of those should leave a hole in the wall.
 *
 * Composed rather than blank, from what the library already knows about the
 * film: its title set in the display serif, the year and the running time under
 * it, and a wash in the film's own colour pair. A fact that is not known is left
 * out rather than shown empty, which is why a film nobody has opened yet shows
 * its title and its year and stops there.
 */

interface ComposedCoverProps {
  film: FilmView;
}

export function ComposedCover({ film }: ComposedCoverProps) {
  const facts = factsOf(film);

  return (
    <span className={styles.cover} aria-hidden="true">
      <span className={styles.title}>{film.title}</span>
      {facts !== null && <span className={styles.facts}>{facts}</span>}
    </span>
  );
}
