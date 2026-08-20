import { type CSSProperties, useCallback, useMemo, useRef, useState } from 'react';
import { motion } from 'motion/react';
import { Button } from '@/shared/ui/Button';
import type { FilmView, Id } from '@/shared/ipc/bindings';
import { nearestFrom, timelineOf } from '@/shared/media/cues';
import { sourceOf, streamOf } from '@/shared/media/source';
import { useNavigation } from '@/app/routes';
import { useFilmAccent, useFilmPalette } from '@/features/library/accent';
import { frameId } from '@/features/library/transition';
import { fileNameOf, useLibrary } from '@/features/library/useLibrary';
import { appearanceOf, comfortOf } from '@/shared/settings/schema';
import { useSettings } from '@/shared/settings/useSettings';
import { REWIND_MS } from './intervals';
import { Controls } from './Controls';
import { Subtitles } from './Subtitles';
import { NEAR_ENOUGH } from './ScrubberPreview';
import { startAtOf } from './resume';
import { useActiveLine } from './useActiveLine';
import { useAlignment } from './useAlignment';
import { useCheck } from './useCheck';
import { useControls } from './useControls';
import { useCues } from './useCues';
import { useFullscreen } from './useFullscreen';
import { useKeepPosition } from './useKeepPosition';
import { usePlayback } from './usePlayback';
import { useShortcuts } from './useShortcuts';
import { useStepping } from './useStepping';
import { offsetLabel, useSync } from './useSync';
import { useTrack } from './useTrack';
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

  // Keyed by the film, so that opening a second one gets a fresh element and
  // fresh state rather than the last film's position.
  return <Film key={film.id} film={film} onBack={back} />;
}

function Film({ film, onBack }: { film: FilmView; onBack: () => void }) {
  const screen = useRef<HTMLDivElement>(null);

  const settings = useSettings((state) => state.settings);
  // Worked out once, and not again when a setting the player also reads
  // changes: the element has already been told where to start.
  const [start] = useState(() =>
    settings.resume === 'beginning' ? 0 : startAtOf(film, REWIND_MS),
  );

  // A film starts a little before where it was left.
  const [video, playback, transport] = usePlayback(film.path, start);
  // Which subtitle is being read comes first: the dialogue and the timing
  // controls are both about that track and not about the film.
  const choice = useTrack(film);
  const dialogue = useCues(choice.active);
  const sync = useSync(choice.active);
  const alignment = useAlignment(choice.active, sync);
  // Built once per film, with the reading comfort preferences folded in as it
  // is built. Nothing downstream of here knows they were applied, and nothing
  // in the frame loop does any of this work again.
  const comfort = useMemo(() => comfortOf(settings), [settings]);
  // How far the lines in hand still have to move. The correction they were read
  // with is already in them, so this is only what has been nudged since and not
  // yet written down, which is nought for a film nobody is adjusting.
  const shift = sync.offsetMs - dialogue.offsetMs;
  const timeline = useMemo(
    () => timelineOf(dialogue.cues, comfort, shift),
    [dialogue.cues, comfort, shift],
  );
  const active = useActiveLine(video, timeline);
  const { visible, wake, hold } = useControls(playback.playing);
  const [fullscreen, toggleFullscreen] = useFullscreen(screen);

  const stepping = useStepping(timeline, transport);
  const [syncing, setSyncing] = useState(false);
  const toggleSync = useCallback(() => {
    setSyncing((showing) => !showing);
  }, []);
  // Asking for an alignment shows the panel, because everything it has to say
  // afterwards is said there. The nudge keys report themselves over the picture
  // and need nothing open; this reports a measurement, a way to stop it and a
  // sentence about how it went, which is more than belongs over a film.
  const { start: measure } = alignment;
  const align = useCallback(() => {
    setSyncing(true);
    measure();
  }, [measure]);
  // Watching what the measurement did, which is the only evidence that is not
  // another number. The lines it goes to are the ones being drawn, so the
  // moment it lands on is a moment in the film rather than in the file.
  const check = useCheck(alignment, timeline.cues, transport);
  const { start: watchIt } = check;
  const see = useCallback(() => {
    setSyncing(true);
    watchIt();
  }, [watchIt]);

  const [choosing, setChoosing] = useState(false);
  const toggleTracks = useCallback(() => {
    setChoosing((showing) => !showing);
  }, []);

  useKeepPosition(film.id, playback.positionMs, playback.playing, playback.durationMs);
  useShortcuts({
    transport,
    stepping,
    sync,
    align,
    check: { available: check.offered, see },
    choice,
    toggleFullscreen,
    toggleSync,
    toggleTracks,
    wake,
  });

  const preview = useMemo(
    () => ({
      source: streamOf(film.path),
      spokenAt: (ms: number) => timeline.cues[nearestFrom(timeline, ms, NEAR_ENOUGH)]?.text ?? null,
    }),
    [film.path, timeline],
  );

  const appearance = useMemo(() => appearanceOf(settings), [settings]);
  const palette = useFilmPalette(film);
  const poster = film.posterPath === null ? undefined : sourceOf(film.posterPath);

  return (
    <div
      ref={screen}
      className={styles.screen}
      style={
        {
          '--film-accent': palette.primary,
          '--film-pair': palette.pair,
        } as CSSProperties
      }
    >
      <motion.div
        layoutId={frameId(film.id)}
        className={styles.frame}
        // The pointer resting still over a playing film is somebody watching
        // it, and the cursor is as much a thing over the picture as the
        // controls are.
        data-idle={!visible}
        onPointerMove={wake}
      >
        <video
          ref={video}
          className={styles.video}
          src={streamOf(film.path)}
          poster={poster}
          preload="metadata"
          playsInline
          onClick={transport.toggle}
        />

        <Subtitles cue={timeline.cues[active] ?? null} appearance={appearance} lifted={visible} />

        {/*
         * What the nudge keys are doing, said over the picture for as long as
         * the value is still being felt for. Somebody adjusting by ear is
         * looking at the film rather than at the control bar, and this is the
         * only way they see the number they are arriving at.
         */}
        {sync.pending && (
          <p className={styles.offset} role="status">
            Subtitles {offsetLabel(sync.offsetMs)}
          </p>
        )}

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
            stepping={stepping}
            preview={preview}
            sync={sync}
            alignment={alignment}
            check={check}
            syncing={syncing}
            choice={choice}
            choosing={choosing}
            visible={visible}
            fullscreen={fullscreen}
            onToggleFullscreen={toggleFullscreen}
            onToggleSync={toggleSync}
            onToggleTracks={toggleTracks}
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
