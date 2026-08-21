import { useEffect, useRef, type CSSProperties, type KeyboardEvent, type RefObject } from 'react';
import { motion } from 'motion/react';
import type { FilmView, Id } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';
import { sourceOf } from '@/shared/media/source';
import { useSetting } from '@/shared/settings/useSettings';
import { Button } from '@/shared/ui/Button';
import { CloseIcon, PlayIcon, SyncIcon } from '@/shared/ui/Icon';
import { classes } from '@/shared/ui/classes';
import { useNavigation } from '@/app/routes';
import { busiestMomentOf } from '@/features/player/check';
import { CHECK, backTo, replacing, said, working } from '@/features/player/outcomes';
import { activeTrackOf, trackNameOf } from '@/features/player/tracks';
import { useAlignment } from '@/features/player/useAlignment';
import { useSync } from '@/features/player/useSync';
import { useFilmPalette } from './accent';
import { ComposedCover } from './ComposedCover';
import { coverNameOf, coverStatementOf, isChosen } from './covers';
import { useCover } from './useCover';
import { fileFactsOf } from './facts';
import { FrameCover } from './FrameCover';
import { useFrames } from './frames';
import { pictureFor } from './picture';
import { remainingOf } from './remaining';
import { resolutionOf, runtimeOf } from './runtime';
import { trackRowsOf } from './trackRows';
import { useLibrary } from './useLibrary';
import { useSheet } from './useSheet';
import { frameId } from './transition';
import styles from './FilmSheet.module.css';

/**
 * A film's page, which is a catalogue card for an object on the disk.
 *
 * Everything on it was written when the film was scanned, so opening it reads
 * nothing from disk and costs a lookup in the library already in memory. That
 * is the whole reason the plate of facts can be this dense: none of it is
 * fetched, and none of it is guessed at either. A fact the file did not state
 * is left out.
 *
 * It is the one screen no streaming service can draw, because it is about the
 * file rather than about the title, and it is set to say so: the values in the
 * figures face and the labels in the interface one, which is what makes a
 * reference read as a reference rather than as a settings dialogue.
 *
 * It comes forward over the library rather than being a screen of its own,
 * because it is a look at one film on the way to watching it and going back
 * should leave the rows exactly where they were.
 */
export function FilmSheet() {
  const filmId = useSheet((sheet) => sheet.filmId);
  const close = useSheet((sheet) => sheet.close);

  // Read out of the library rather than held, so a scan that changes the row
  // underneath an open sheet changes what the sheet says.
  const film = useLibrary((library) =>
    filmId === null ? null : (library.films.find((known) => known.id === filmId) ?? null),
  );

  if (film === null) return null;
  return <Panel film={film} onClose={close} />;
}

function Panel({ film, onClose }: { film: FilmView; onClose: () => void }) {
  const openFilm = useNavigation((navigation) => navigation.openFilm);
  const palette = useFilmPalette(film);
  const panel = useRef<HTMLDivElement>(null);

  useReturningFocus(panel);

  // Behind the card, where a wide picture is wanted: the frame from where the
  // film was stopped if one has been taken, and the poster otherwise.
  const frame = useFrames((held) => held.frames[film.id]?.url);
  const still = frame ?? (film.posterPath === null ? null : sourceOf(film.posterPath));

  // The cover itself is whatever the tile is showing, because the tile morphs
  // into it. Two different pictures for one film would be visible in the
  // moment between them.
  const picture = pictureFor(film, useSetting('withoutArtwork'));
  const carryingOn = film.position !== null;

  const play = () => {
    onClose();
    openFilm(film.id);
  };

  // The same check the player offers, from the one place that has no player in
  // it. There is nothing to watch a measurement land on here, so this goes to
  // where there is, at the moment the check would have played.
  const watch = (atMs: number) => {
    onClose();
    openFilm(film.id, atMs);
  };

  // Taken here so that the shell does not also read it as leaving the screen
  // behind the sheet.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    onClose();
  };

  return (
    <div
      className={styles.over}
      style={
        {
          '--film-accent': palette.primary,
          '--film-pair': palette.pair,
        } as CSSProperties
      }
    >
      {/* The way out for a pointer. The keyboard has Escape, and the close
          button below is in the tab order for both. */}
      <div className={styles.scrim} onClick={onClose} aria-hidden="true" />

      <div
        className={styles.sheet}
        // Named so that a picture dropped anywhere on the page becomes this
        // film's cover, the same way one dropped on its tile does.
        data-film-id={film.id}
        role="dialog"
        aria-modal="true"
        aria-labelledby={`film-sheet-${String(film.id)}`}
        tabIndex={-1}
        ref={panel}
        onKeyDown={onKeyDown}
      >
        {/* Outside the scrolling area, so it stays where it was put on a page
            long enough to scroll. */}
        <button type="button" className={styles.close} aria-label="Close" onClick={onClose}>
          <CloseIcon size={12} />
        </button>

        <div className={styles.scroll}>
          <span className={styles.art} aria-hidden="true">
            {still !== null && <img className={styles.still} src={still} alt="" />}
          </span>

          <article className={styles.card}>
            <div className={styles.aside}>
              <motion.span layoutId={frameId(film.id)} className={styles.cover}>
                {picture.kind === 'composed' && <ComposedCover film={film} />}
                {picture.kind === 'frame' && (
                  <FrameCover film={film} src={sourceOf(picture.path)} />
                )}
                {picture.kind === 'artwork' && (
                  <img
                    className={styles.still}
                    src={sourceOf(picture.path)}
                    alt=""
                    draggable={false}
                  />
                )}
              </motion.span>

              <CoverChoice film={film} />
            </div>

            <div className={styles.main}>
              <h2 className={styles.title} id={`film-sheet-${String(film.id)}`}>
                {film.title}
              </h2>

              <Facts film={film} />

              {film.position !== null && (
                <div className={styles.resume}>
                  <span className={styles.bar} aria-hidden="true">
                    <span style={{ width: `${String(watchedPercent(film))}%` }} />
                  </span>
                  <small>
                    {remainingOf(film)} · {String(watchedPercent(film))}% watched
                  </small>
                </div>
              )}

              <div className={styles.actions}>
                {film.missing ? (
                  <p className={styles.gone} role="status">
                    The file is not where it was. Reconnect the drive it is on and Subtext will find
                    it again, with where you stopped still on it.
                  </p>
                ) : (
                  <>
                    <Button tone="primary" onClick={play}>
                      <PlayIcon size={13} />
                      {carryingOn ? 'Carry on' : 'Play'}
                    </Button>
                    <ShowInFolder film={film} />
                  </>
                )}
              </div>

              <section className={styles.section}>
                <h3 className={styles.heading}>What the file is</h3>
                <FilePlate film={film} />
              </section>

              <section className={styles.section}>
                <h3 className={styles.heading}>Tracks</h3>
                <Tracks film={film} />
              </section>

              <AlignOffer film={film} onWatch={watch} />
            </div>
          </article>
        </div>
      </div>
    </div>
  );
}

/**
 * The line under the title: the year, how long it runs and what the picture is.
 *
 * The shorthand rather than the figures, since the plate below carries the
 * exact ones. A film nobody has opened and whose container was never read knows
 * none of the three, and then the line is left out rather than drawn empty.
 */
function Facts({ film }: { film: FilmView }) {
  const facts = factsOf(film);
  if (facts.length === 0) return null;

  return (
    <p className={styles.under}>
      {facts.map((fact, at) => (
        <span key={fact} className={styles.fact}>
          {at > 0 && <i className={styles.dot} aria-hidden="true" />}
          {fact}
        </span>
      ))}
    </p>
  );
}

/**
 * What the file is, as a plate of values.
 *
 * Every value in the figures face and every label in the interface one, which
 * is the whole of why this reads as a catalogue entry. A fact the file did not
 * state has no row at all, so an MP4, which this application does not parse,
 * shows a shorter plate rather than a full one with holes in it.
 *
 * The path takes the width, because it is the one value long enough that a
 * column would leave nothing of it but the drive.
 */
function FilePlate({ film }: { film: FilmView }) {
  return (
    <dl className={styles.plate}>
      {fileFactsOf(film).map((row) => (
        <div className={classes(styles.value, row.path === true && styles.wide)} key={row.label}>
          <dt>{row.label}</dt>
          <dd
            className={classes(row.path === true && styles.path)}
            title={row.path === true ? row.value : undefined}
          >
            {row.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}

/**
 * Everything the film carries, sound and subtitles in one table.
 *
 * Reading them against each other is the point: a Japanese film with English
 * subtitles beside it is one glance down two columns, and two separate blocks
 * would make it a scroll. A cell the film view does not carry is empty, in the
 * same way a fact the file did not state has no row in the plate above.
 *
 * A kind with no rows at all is said under the table rather than left to be
 * noticed, because an absence is what somebody looking for subtitles, or
 * wondering why a film has no sound, came to the table to find out.
 */
function Tracks({ film }: { film: FilmView }) {
  const rows = trackRowsOf(film, useSetting('subtitleLanguage'));

  if (rows.length === 0) return <p className={styles.none}>Nothing recorded</p>;

  const missing = [
    rows.some((row) => row.kind === 'Sound') ? null : 'No sound recorded',
    rows.some((row) => row.kind === 'Subtitle') ? null : 'No subtitles found',
  ].filter((note) => note !== null);

  return (
    <>
      <table className={styles.tracks}>
        <thead>
          <tr>
            <th scope="col">Kind</th>
            <th scope="col">Language</th>
            <th scope="col">Codec</th>
            <th scope="col">What it is</th>
            {/* The state, which is a mark rather than a value and reads better
                without a heading over a column that is mostly empty. */}
            <th scope="col">
              <span className={styles.away}>State</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.key} data-in-use={row.state !== null && !row.state.warn}>
              <td>{row.kind}</td>
              <td>{row.language}</td>
              <td className={styles.mono}>{row.codec}</td>
              <td>{row.what}</td>
              <td>
                {row.state !== null && (
                  <span className={classes(styles.pill, row.state.warn && styles.warn)}>
                    {row.state.name}
                  </span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {missing.map((note) => (
        <p className={styles.none} key={note}>
          {note}
        </p>
      ))}
    </>
  );
}

/**
 * Where the picture came from, and the two ways to change it.
 *
 * Under the cover because that is what it is about. A film page that shows an
 * image without saying what kind of claim it is leaves somebody guessing at
 * whether Subtext found the artwork or made a frame up, and those are very
 * different things to be looking at.
 *
 * Putting a choice back is offered only where there is one, since a cover the
 * scan chose is already the scan's to change and offering to hand it back
 * would be offering to do nothing.
 */
function CoverChoice({ film }: { film: FilmView }) {
  const cover = useCover(film);

  return (
    <div className={styles.source}>
      <p className={styles.sourceName}>
        <i className={styles.mark} aria-hidden="true" />
        {coverNameOf(film.coverSource)}
      </p>
      <p className={styles.sourceNote}>{coverStatementOf(film.coverSource)}</p>

      <div className={styles.sourceActions}>
        <Button tone="ghost" disabled={cover.busy} onClick={cover.choose}>
          Choose an image
        </Button>
        {isChosen(film.coverSource) && (
          <Button tone="ghost" disabled={cover.busy} onClick={cover.clear}>
            Use what was found
          </Button>
        )}
      </div>

      {cover.problem !== null && (
        <p className={styles.sourceProblem} role="status">
          {cover.problem}
        </p>
      )}
    </div>
  );
}

/**
 * Working the subtitle timing out by listening to the film, from here as well
 * as from the player.
 *
 * The same command, the same threshold and the same wording as the timing
 * panel. This is a second door onto the room rather than a second room: the
 * moment somebody is looking at a film's page is a moment they might already
 * know its subtitles are out, and being made to start the film to say so is a
 * detour.
 */
function AlignOffer({ film, onWatch }: { film: FilmView; onWatch: (atMs: number) => void }) {
  const preferred = useSetting('subtitleLanguage');
  // The track this film would be watched with, and only that one. Falling back
  // to the first readable track when subtitles have been turned off would offer
  // to measure a subtitle the player is not going to draw, which is a
  // measurement nobody could check and a correction written to a track nobody
  // asked about.
  const track = activeTrackOf(film, preferred);

  const sync = useSync(track);
  const alignment = useAlignment(track, sync);
  const { state } = alignment;

  // A film with only pictures for subtitles has nothing to measure, and saying
  // so is the table's job rather than an offer that would decline.
  if (track === null || film.missing) return null;

  if (state.phase === 'confirming') {
    return (
      <div className={styles.align}>
        <p className={styles.offer}>{replacing(sync.offsetMs)}</p>
        <div className={styles.row}>
          <Button onClick={alignment.confirm}>Measure anyway</Button>
          <Button tone="ghost" onClick={alignment.dismiss}>
            Keep mine
          </Button>
        </div>
      </div>
    );
  }

  if (state.phase === 'running') {
    return (
      <div className={styles.align}>
        <div className={styles.row}>
          <p className={styles.working} aria-live="polite">
            {working(state.stage)}
          </p>
          <Button tone="ghost" onClick={alignment.cancel}>
            Stop
          </Button>
        </div>
        <div
          className={styles.progress}
          role="progressbar"
          aria-label="Listening to the film"
          aria-valuenow={Math.round(state.fraction * 100)}
        >
          <div className={styles.done} style={{ scale: `${String(state.fraction)} 1` }} />
        </div>
      </div>
    );
  }

  if (state.phase === 'outcome') {
    const { title, sentence } = said(state.outcome);
    const written = state.outcome.outcome === 'aligned' && !state.undone ? state.outcome : null;

    return (
      <div className={styles.align}>
        <p className={styles.offerTitle}>{title}</p>
        <p className={styles.offer} role="status">
          {sentence}
        </p>
        {written !== null && <p className={styles.offer}>{CHECK.note}</p>}
        <div className={styles.row}>
          {written !== null && (
            <>
              <Button
                onClick={() => {
                  void momentIn(track.id).then(onWatch, () => {
                    // The lines could not be read, so there is no busiest
                    // moment to go to. Nothing is said: the measurement stands,
                    // the sentence about it is still on screen, and the film
                    // can be opened with the button above it.
                  });
                }}
              >
                {CHECK.offer}
              </Button>
              <Button tone="ghost" onClick={alignment.undo}>
                Put it back {backTo(written.previous.offsetMs)}
              </Button>
            </>
          )}
          <Button tone="ghost" onClick={alignment.dismiss}>
            Dismiss
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.align}>
      <p className={styles.offerTitle}>
        <SyncIcon size={14} />
        Timing looks off?
      </p>
      <p className={styles.offer}>
        Subtext reads the soundtrack, works out where the talking falls, and moves the subtitles to
        match. It takes a few seconds, and the film need not be open. It measures{' '}
        {trackNameOf(track)}, which is the subtitle this film is watched with.
      </p>
      <div className={styles.row}>
        <Button onClick={alignment.start}>Listen and line up</Button>
      </div>
    </div>
  );
}

/**
 * The moment in a film worth watching to see whether a measurement landed.
 *
 * Read here rather than held, because a film's page shows what was scanned and
 * reads nothing from disk to do it. The lines of a track are a few hundred
 * kilobytes and are wanted at the moment somebody asks to watch, which is once
 * per measurement rather than once per film looked at.
 */
async function momentIn(trackId: Id): Promise<number> {
  const cues = await ipc.trackCues(trackId);
  const at = busiestMomentOf(cues);
  if (at === null) throw new Error('this track has no lines to watch for');
  return at;
}

/** Opening the folder the film is in, with the film picked out. */
function ShowInFolder({ film }: { film: FilmView }) {
  return (
    <Button
      onClick={() => {
        void ipc.showInFolder(film.id).catch(() => {
          // A machine with no file manager, or a drive that has gone since the
          // sheet was opened. Nothing was going to happen on this screen
          // either way, and the library says the film is missing soon enough.
        });
      }}
    >
      Show in folder
    </Button>
  );
}

/** The three, leaving out whichever of them the film does not know. */
function factsOf(film: FilmView): string[] {
  const video = film.details?.video;
  const said = [resolutionOf(film), video?.codec].filter((part) => part != null).join(' ');
  const picture = said === '' ? null : said;

  return [
    film.year === null ? null : String(film.year),
    runtimeOf(film.durationMs),
    picture,
  ].filter((part) => part !== null);
}

/** How far through the film somebody is, as a whole number. */
function watchedPercent(film: FilmView): number {
  const position = film.position;
  if (position === null) return 0;
  if (position.finished) return 100;
  return Math.round((position.progress ?? 0) * 100);
}

/**
 * Focus into the sheet when it opens, and back where it came from when it goes.
 *
 * A sheet that opens without taking focus leaves somebody on the keyboard
 * tabbing through the library behind it, and one that closes without giving it
 * back drops them at the top of the window rather than at the tile they were
 * on.
 */
function useReturningFocus(panel: RefObject<HTMLDivElement | null>) {
  useEffect(() => {
    const opener = document.activeElement;
    panel.current?.focus();

    return () => {
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, [panel]);
}
