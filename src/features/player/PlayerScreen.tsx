import type { CSSProperties } from 'react';
import { motion } from 'motion/react';
import { Button } from '@/shared/ui/Button';
import type { Id } from '@/shared/ipc/bindings';
import { sourceOf } from '@/shared/media/source';
import { useNavigation } from '@/app/routes';
import { useFilmAccent } from '@/features/library/accent';
import { fallbackFor, paletteOf } from '@/features/library/fallback';
import { frameId } from '@/features/library/transition';
import { fileNameOf, linesOf, useLibrary } from '@/features/library/useLibrary';
import styles from './PlayerScreen.module.css';

interface PlayerScreenProps {
  filmId: Id;
}

/**
 * Where the film will play.
 *
 * Playback, the control bar and the transcript are the next pieces of work.
 * What stands in for the film is the frame the poster grew out of, carrying the
 * same name as the tile it came from, so opening a film is one frame growing
 * into the room where it will play rather than a screen being replaced.
 */
export function PlayerScreen({ filmId }: PlayerScreenProps) {
  const film = useLibrary((library) => library.films.find((known) => known.id === filmId));
  const back = useNavigation((navigation) => navigation.back);

  useFilmAccent(film?.accent ?? null);

  if (!film) {
    return (
      <div className={styles.screen}>
        <p className={styles.note}>That film is no longer in the library.</p>
        <Button onClick={back}>Back to your films</Button>
      </div>
    );
  }

  const palette = paletteOf(film);
  const poster = film.posterPath === null ? null : sourceOf(film.posterPath);

  return (
    <div
      className={styles.screen}
      style={
        {
          '--film-accent': palette.primary,
          '--film-pair': palette.pair,
        } as CSSProperties
      }
    >
      <motion.div layoutId={frameId(film.id)} className={styles.frame}>
        {poster === null ? (
          <span className={styles.generated} aria-hidden="true">
            {fallbackFor(film.title).initials}
          </span>
        ) : (
          <img className={styles.still} src={poster} alt="" draggable={false} />
        )}
      </motion.div>

      <div className={styles.detail}>
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
    </div>
  );
}
