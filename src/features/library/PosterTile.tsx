import { useEffect, useState, type CSSProperties } from 'react';
import { motion } from 'motion/react';
import type { FilmView } from '@/shared/ipc/bindings';
import { sourceOf } from '@/shared/media/source';
import { classes } from '@/shared/ui/classes';
import { fallbackFor, paletteOf } from './fallback';
import { frameId, stillnessWanted } from './transition';
import { linesOf } from './useLibrary';
import styles from './PosterTile.module.css';

/**
 * One film in the grid.
 *
 * The tile carries its own colours as custom properties, which is what lets the
 * lift, the glow and the ring all be one film's colour without a component
 * anywhere reading a palette. A film with no captured frame draws the
 * composition made from its title instead, so the grid is never a row of holes.
 */

/** How long a pointer rests on a tile before the preview starts. */
const DWELL = 650;

interface PosterTileProps {
  film: FilmView;
  onOpen: (film: FilmView) => void;
}

export function PosterTile({ film, onOpen }: PosterTileProps) {
  const [hovered, setHovered] = useState(false);
  const preview = usePreview(hovered && !film.missing, film.path);

  const palette = paletteOf(film);
  const still = film.posterPath === null ? null : sourceOf(film.posterPath);
  const progress = film.position?.progress ?? null;

  return (
    <button
      type="button"
      className={classes(styles.tile, film.missing && styles.gone)}
      style={
        {
          '--film-accent': palette.primary,
          '--film-pair': palette.pair,
        } as CSSProperties
      }
      onClick={() => {
        onOpen(film);
      }}
      onPointerEnter={() => {
        setHovered(true);
      }}
      onPointerLeave={() => {
        setHovered(false);
      }}
      onFocus={() => {
        setHovered(true);
      }}
      onBlur={() => {
        setHovered(false);
      }}
    >
      <motion.span layoutId={frameId(film.id)} className={styles.frame}>
        {still === null ? (
          <span className={styles.generated} aria-hidden="true">
            {fallbackFor(film.title).initials}
          </span>
        ) : (
          <img className={styles.still} src={still} alt="" draggable={false} />
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
  );
}

/** What sits under the title: the year, and what there is to search. */
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
