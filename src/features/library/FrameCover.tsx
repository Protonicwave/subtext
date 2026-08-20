import type { FilmView } from '@/shared/ipc/bindings';
import { classes } from '@/shared/ui/classes';
import styles from './FrameCover.module.css';

/**
 * A frame from the film, drawn where a cover goes.
 *
 * A frame is sixteen by nine and a cover is two by three, so one of them has to
 * give. Cutting the frame down the middle throws away half the width of the
 * shot it was taken from and leaves something that looks like a cover that went
 * wrong. Letterboxing it keeps the whole shot, and the room left over is filled
 * with the frame itself, blurred, so the tile is still the colour of the film
 * rather than a black bar above and below.
 *
 * The title is set over it because a frame does not carry one the way a poster
 * does. A frame that admits to being a frame is a better answer than a frame
 * pretending to be artwork. It is left off where the title is already set
 * beside the picture, since the masthead names the film in its own heading and
 * two of them would be the same word twice.
 *
 * The same image twice, which costs one decode: the second use of a URL is the
 * first one's bitmap.
 */

interface FrameCoverProps {
  film: FilmView;
  /** Where the poster drawn from the frame is served from. */
  src: string;
  /** Whether the film's title is set over it. */
  titled?: boolean;
}

export function FrameCover({ film, src, titled = true }: FrameCoverProps) {
  return (
    <span className={classes(styles.cover, titled && styles.titled)} aria-hidden="true">
      <img className={styles.wash} src={src} alt="" draggable={false} />
      <img className={styles.shot} src={src} alt="" draggable={false} />
      {titled && <span className={styles.title}>{film.title}</span>}
    </span>
  );
}
