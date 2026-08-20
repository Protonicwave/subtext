import { useEffect, useState, type CSSProperties } from 'react';
import { motion } from 'motion/react';
import type { FilmView } from '@/shared/ipc/bindings';
import { sourceOf } from '@/shared/media/source';
import { useSetting } from '@/shared/settings/useSettings';
import { classes } from '@/shared/ui/classes';
import { stillnessWanted } from '@/shared/ui/stillness';
import { useFilmPalette } from './accent';
import { ComposedCover } from './ComposedCover';
import { CoverMark } from './CoverMark';
import { FrameCover } from './FrameCover';
import { pictureFor } from './picture';
import { frameId } from './transition';
import { linesOf } from './useLibrary';
import styles from './PosterTile.module.css';

/**
 * One film in the grid.
 *
 * The tile carries its own colours as custom properties, which is what lets the
 * lift, the glow and the ring all be one film's colour without a component
 * anywhere reading a palette. A film with no artwork on the disk draws the
 * cover composed from what is known about it, so the grid is never a row of
 * holes and never a wall of frames nobody chose.
 */

/** How long a pointer rests on a tile before the preview starts. */
const DWELL = 650;

interface PosterTileProps {
  film: FilmView;
  onOpen: (film: FilmView) => void;
  /**
   * Whether this tile is the one that moves into the player.
   *
   * The same film can be on the screen twice, once on the row of what to carry
   * on with and once in its own folder, and a shared element has to be one
   * element. The film's own row is the one that travels.
   */
  shared?: boolean;
}

export function PosterTile({ film, onOpen, shared = true }: PosterTileProps) {
  const [hovered, setHovered] = useState(false);
  const preview = usePreview(hovered && !film.missing, film.path);

  const palette = useFilmPalette(film);
  const picture = pictureFor(film, useSetting('withoutArtwork'));
  const progress = film.position?.progress ?? null;

  return (
    // The film is named on the tile itself rather than held anywhere, so that a
    // picture dropped on it can be matched to it without every tile in a wall
    // of two thousand having to be a listener.
    <div
      className={classes(styles.tile, film.missing && styles.gone)}
      data-film-id={film.id}
      style={
        {
          '--film-accent': palette.primary,
          '--film-pair': palette.pair,
        } as CSSProperties
      }
      onPointerEnter={() => {
        setHovered(true);
      }}
      onPointerLeave={() => {
        setHovered(false);
      }}
    >
      <button
        type="button"
        className={styles.open}
        onClick={() => {
          onOpen(film);
        }}
        onFocus={() => {
          setHovered(true);
        }}
        onBlur={() => {
          setHovered(false);
        }}
      >
        <motion.span {...(shared && { layoutId: frameId(film.id) })} className={styles.frame}>
          {picture.kind === 'composed' && <ComposedCover film={film} />}
          {picture.kind === 'frame' && <FrameCover film={film} src={sourceOf(picture.path)} />}
          {picture.kind === 'artwork' && (
            <img className={styles.still} src={sourceOf(picture.path)} alt="" draggable={false} />
          )}

          {preview !== null && (
            <video
              className={styles.preview}
              src={preview}
              muted
              autoPlay
              loop
              playsInline
              disablePictureInPicture
              crossOrigin="anonymous"
              onLoadedMetadata={(event) => {
                // From the same moment the poster was taken, so the preview
                // carries on from the still rather than starting at the logos.
                const { duration } = event.currentTarget;
                if (Number.isFinite(duration)) event.currentTarget.currentTime = duration * 0.2;
              }}
            />
          )}

          {film.missing && <span className={styles.badge}>Missing</span>}

          {progress !== null && (
            <span className={styles.sliver} aria-hidden="true">
              <span style={{ width: `${String(Math.round(progress * 100))}%` }} />
            </span>
          )}
        </motion.span>

        <span className={styles.caption}>
          <span className={styles.name}>{film.title}</span>
          <span className={styles.meta}>{describe(film)}</span>
        </span>
      </button>

      {/* A film whose file is not there has no picture to settle, and saying
          where the one on screen came from would be the least of what is wrong
          with it. */}
      {!film.missing && <CoverMark film={film} shown={hovered} />}
    </div>
  );
}

/** What sits under the title: the year, and how much dialogue was read. */
function describe(film: FilmView): string {
  const year = film.year === null ? null : String(film.year);
  const lines =
    film.tracks.length === 0 ? 'No subtitles' : `${linesOf(film).toLocaleString('en-GB')} lines`;

  return [year, film.missing ? 'Missing' : lines].filter((part) => part !== null).join(' · ');
}

/**
 * The silent preview, once a pointer has rested long enough to mean it.
 *
 * The delay is the whole feature. Without it, crossing the grid on the way
 * somewhere else starts and stops a dozen decoders, which is both a stutter and
 * a lot of work for something nobody asked to see. Somebody who has stopped on
 * a tile has asked.
 */
function usePreview(wanted: boolean, path: string): string | null {
  const [playing, setPlaying] = useState(false);

  useEffect(() => {
    // Somebody who has asked for less movement has asked for this too: a tile
    // that starts playing under the pointer is the most of it on this screen.
    if (!wanted || stillnessWanted()) return;

    const timer = setTimeout(() => {
      setPlaying(true);
    }, DWELL);

    return () => {
      clearTimeout(timer);
      setPlaying(false);
    };
  }, [wanted]);

  return playing ? sourceOf(path) : null;
}
