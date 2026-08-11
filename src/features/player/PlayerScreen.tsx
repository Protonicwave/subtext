import { Button } from '@/shared/ui/Button';
import type { Id } from '@/shared/ipc/bindings';
import { useNavigation } from '@/app/routes';
import { fileNameOf, linesOf, useLibrary } from '@/features/library/useLibrary';
import styles from './PlayerScreen.module.css';

interface PlayerScreenProps {
  filmId: Id;
}

/**
 * Where the film will play.
 *
 * Playback, the control bar and the transcript are the next pieces of work.
 * What this shows in the meantime is what the library knows about the film,
 * which is the part that already works, and says plainly what does not.
 */
export function PlayerScreen({ filmId }: PlayerScreenProps) {
  const film = useLibrary((library) => library.films.find((known) => known.id === filmId));
  const back = useNavigation((navigation) => navigation.back);

  if (!film) {
    return (
      <div className={styles.screen}>
        <p className={styles.note}>That film is no longer in the library.</p>
        <Button onClick={back}>Back to your films</Button>
      </div>
    );
  }

  return (
    <div className={styles.screen}>
      <h1 className={styles.title}>
        {film.title}
        {film.year !== null && <em> {film.year}</em>}
      </h1>
      <p className={styles.path} data-selectable>
        {fileNameOf(film.path)}
      </p>
      <p className={styles.note}>
        {film.tracks.length === 0
          ? 'No subtitles were paired with this film, so it will have no transcript.'
          : `${linesOf(film).toLocaleString('en-GB')} lines of dialogue are indexed and ready.`}
      </p>
      <p className={styles.note}>Playback itself is not built yet.</p>
      <Button onClick={back}>Back to your films</Button>
    </div>
  );
}
