import { type CSSProperties, useRef } from 'react';
import { motion } from 'motion/react';
import { Button } from '@/shared/ui/Button';
import type { FilmView, Id } from '@/shared/ipc/bindings';
import { sourceOf, streamOf } from '@/shared/media/source';
import { useNavigation } from '@/app/routes';
import { useFilmAccent } from '@/features/library/accent';
import { paletteOf } from '@/features/library/fallback';
import { frameId } from '@/features/library/transition';
import { fileNameOf, useLibrary } from '@/features/library/useLibrary';
import { Controls } from './Controls';
import { Subtitles } from './Subtitles';
import { PLAYBACK, SUBTITLES } from './defaults';
import { startAtOf } from './resume';
import { useActiveCue } from './useActiveCue';
import { useControls } from './useControls';
import { useCues } from './useCues';
import { useFullscreen } from './useFullscreen';
import { useKeepPosition } from './useKeepPosition';
import { usePlayback } from './usePlayback';
import { useShortcuts } from './useShortcuts';
import styles from './PlayerScreen.module.css';

/**
 * The film, playing.
 *
 * The picture is the platform's video element pointed at the stream protocol,
 * and everything over it is ours: the subtitles, the control bar and the wash
 * they sit on. The element keeps the position and decides what it can decode,
 * which is why nothing here holds a second copy of either.
 *
 * The frame is the same element as the tile in the library as far as the
 * animation is concerned, so opening a film is a poster growing into a picture
 * rather than one screen replacing another.
 */

interface PlayerScreenProps {
  filmId: Id;
}

export function PlayerScreen({ filmId }: PlayerScreenProps) {
  const film = useLibrary((library) => library.films.find((known) => known.id === filmId));
  const back = useNavigation((navigation) => navigation.back);

  useFilmAccent(film?.accent ?? null);

  if (film === undefined) {
    return (
      <div className={styles.missing}>
        <p className={styles.note}>That film is no longer in the library.</p>
        <Button onClick={back}>Back to your films</Button>
      </div>
    );
  }

  if (film.missing) {
    return (
      <div className={styles.missing}>
        <h1 className={styles.title}>{film.title}</h1>
        <p className={styles.note}>
          This file is not where it was. Subtext has kept where you were up to, so reconnecting the
          drive it is on is all it needs.
        </p>
        <p className={styles.path}>{film.path}</p>
        <Button onClick={back}>Back to your films</Button>
      </div>
    );
  }

  // Keyed by the film, so that opening a second one from search later gets a
  // fresh element and fresh state rather than the last film's position.
  return <Film key={film.id} film={film} onBack={back} />;
}

function Film({ film, onBack }: { film: FilmView; onBack: () => void }) {
  const screen = useRef<HTMLDivElement>(null);

  const [video, playback, transport] = usePlayback(film.path, startAtOf(film, PLAYBACK.rewindMs));
  const cues = useCues(film);
  const cue = useActiveCue(video, cues);
  const { visible, wake, hold } = useControls(playback.playing);
  const [fullscreen, toggleFullscreen] = useFullscreen(screen);

  useKeepPosition(film.id, playback.positionMs, playback.playing, playback.durationMs);
  useShortcuts(transport, toggleFullscreen, wake);

  const palette = paletteOf(film);
  const poster = film.posterPath === null ? undefined : sourceOf(film.posterPath);

  return (
    <div
      ref={screen}
      className={styles.screen}
      // The pointer resting still over a playing film is somebody watching it,
      // and the cursor is as much a thing over the picture as the controls are.
      data-idle={!visible}
      onPointerMove={wake}
      style={
        {
          '--film-accent': palette.primary,
          '--film-pair': palette.pair,
        } as CSSProperties
      }
    >
      <motion.div layoutId={frameId(film.id)} className={styles.frame}>
        <video
          ref={video}
          className={styles.video}
          src={streamOf(film.path)}
          poster={poster}
          preload="metadata"
          playsInline
          onClick={transport.toggle}
        />

        <Subtitles cue={cue} appearance={SUBTITLES} lifted={visible} />

        {/*
         * Opening a film off a drive that has been asleep takes a moment, and
         * so does a seek into a part of it that has not been read. Without
         * this the window looks as though it has stopped.
         */}
        {playback.problem === null && (!playback.ready || playback.waiting) && (
          <div className={styles.opening} role="status" aria-label="Opening the film" />
        )}

        {playback.problem === null ? (
          <Controls
            playback={playback}
            transport={transport}
            visible={visible}
            fullscreen={fullscreen}
            onToggleFullscreen={toggleFullscreen}
            onHold={hold}
          />
        ) : (
          <div className={styles.refused} role="alert">
            <h2 className={styles.refusedTitle}>This film will not play</h2>
            <p className={styles.note}>{playback.problem}</p>
            <p className={styles.path}>{fileNameOf(film.path)}</p>
            <Button onClick={onBack}>Back to your films</Button>
          </div>
        )}
      </motion.div>
    </div>
  );
}
