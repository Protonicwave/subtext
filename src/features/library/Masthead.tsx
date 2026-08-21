import type { CSSProperties } from 'react';
import type { FilmView } from '@/shared/ipc/bindings';
import { clockOf } from '@/shared/media/clock';
import { sourceOf } from '@/shared/media/source';
import { useSetting } from '@/shared/settings/useSettings';
import { Button } from '@/shared/ui/Button';
import { PlayIcon } from '@/shared/ui/Icon';
import { useFilmAccent, useFilmPalette } from './accent';
import { ComposedCover } from './ComposedCover';
import { FrameCover } from './FrameCover';
import { useFrames } from './frames';
import { pictureFor } from './picture';
import { resolutionOf, runtimeOf } from './runtime';
import { leftOf, stoppedOn, watchedWithOf } from './stopped';
import { useStoppedLine } from './useStoppedLine';
import styles from './Masthead.module.css';

/**
 * What the library opens on: where the reader stopped.
 *
 * Not an offer. A streaming service shows one title large because it has twenty
 * thousand the reader has not seen and something to gain by their picking one.
 * This library is a few hundred films in folders somebody made, on a disk they
 * own, and a layout borrowed from a shop makes a small collection look empty
 * rather than chosen.
 *
 * So it states: how much is left of the film in progress, when it was stopped,
 * what it is being watched with, and the line that was on screen at that
 * moment. The picture sits beside the words at the size of a cover rather than
 * filling the window, which is what lets the shelves start high enough that
 * some of the library is visible without scrolling.
 *
 * The window takes the film's colours while it is shown, as it does in the
 * player, so the light behind the library is the light of the film being
 * carried on with.
 */

interface MastheadProps {
  film: FilmView;
  /** Watch it, from where it was left. */
  onPlay: (film: FilmView) => void;
  /** Read what it is first, which is the film's own page. */
  onOpen: (film: FilmView) => void;
}

export function Masthead({ film, onPlay, onOpen }: MastheadProps) {
  const palette = useFilmPalette(film);
  useFilmAccent(film.accent);

  const preferred = useSetting('subtitleLanguage');
  const line = useStoppedLine(film);
  const position = film.position;
  const progress = position?.progress ?? null;

  return (
    <section
      className={styles.masthead}
      aria-labelledby="masthead-title"
      style={
        {
          '--film-accent': palette.primary,
          '--film-pair': palette.pair,
        } as CSSProperties
      }
    >
      <span className={styles.art}>
        <Picture film={film} />
      </span>

      <div className={styles.body}>
        <p className={styles.kicker}>
          <i className={styles.rule} aria-hidden="true" />
          {position === null ? 'From your library' : 'You stopped here'}
        </p>

        {/*
         * The line, where the film has one to give. It is left out silently
         * rather than replaced with a note about why: a film with no subtitles
         * is not a fault to report on the screen somebody opens the window to.
         */}
        {line !== null && (
          <p className={styles.said}>
            <span className={styles.at}>{clockOf(line.startMs)}</span>
            <q className={styles.line}>{line.text}</q>
          </p>
        )}

        <h1 className={styles.title} id="masthead-title">
          {film.title}
        </h1>

        <p className={styles.facts}>
          {factsOf(film, preferred).map((fact, at) => (
            <span key={fact} className={styles.fact}>
              {at > 0 && <i className={styles.dot} aria-hidden="true" />}
              {fact}
            </span>
          ))}
        </p>

        {progress !== null && (
          <span className={styles.progress} aria-hidden="true">
            <span
              className={styles.done}
              style={{ width: `${String(Math.round(progress * 100))}%` }}
            />
          </span>
        )}

        <div className={styles.actions}>
          <Button
            tone="primary"
            onClick={() => {
              onPlay(film);
            }}
          >
            <PlayIcon size={13} />
            {position === null ? 'Play' : 'Carry on'}
          </Button>
          <Button
            onClick={() => {
              onOpen(film);
            }}
          >
            Details
          </Button>
        </div>
      </div>
    </section>
  );
}

/**
 * The picture beside the words.
 *
 * The frame from where the film was stopped where one has been taken, since
 * that moment is what the rest of the masthead is about. Failing that, whatever
 * the film's own tile draws, so the two cannot show different pictures for the
 * same film.
 *
 * A frame keeps its own shape here as it does everywhere else, letterboxed on a
 * blurred extension of itself rather than cut to two by three, and without its
 * title over it: the heading beside it has already said which film this is.
 */
function Picture({ film }: { film: FilmView }) {
  const frame = useFrames((held) => held.frames[film.id]?.url);
  const picture = pictureFor(film, useSetting('withoutArtwork'));

  if (frame !== undefined) return <FrameCover film={film} src={frame} titled={false} />;

  switch (picture.kind) {
    case 'artwork':
      return <img className={styles.still} src={sourceOf(picture.path)} alt="" draggable={false} />;
    case 'frame':
      return <FrameCover film={film} src={sourceOf(picture.path)} titled={false} />;
    case 'composed':
      return <ComposedCover film={film} />;
  }
}

/**
 * What is said under the title, leaving out whatever is not known.
 *
 * A film in progress is described by where the reader got to: how much is left
 * of how long, when they stopped, and what they were watching it with. A film
 * nobody has started has none of those, so it is described by what it is, which
 * is the year, the running time and the size of the picture.
 *
 * Either way a fact that is not known is a gap rather than a nought. A film
 * nobody has opened has no running time, a file with no year in its name has no
 * year, and a film in a container this application does not parse has no
 * resolution.
 */
function factsOf(film: FilmView, preferred: string): string[] {
  const facts =
    film.position === null
      ? [runtimeOf(film.durationMs), resolutionOf(film)]
      : [leftOf(film), stoppedOn(film.position.updatedAt), watchedWithOf(film, preferred)];

  return [film.year === null ? null : String(film.year), ...facts].filter((fact) => fact !== null);
}
