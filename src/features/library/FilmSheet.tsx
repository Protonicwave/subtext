import {
  useEffect,
  useRef,
  type CSSProperties,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
} from 'react';
import { motion } from 'motion/react';
import type { AudioView, FilmView, TrackView } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';
import { sourceOf } from '@/shared/media/source';
import { useSetting } from '@/shared/settings/useSettings';
import { Button } from '@/shared/ui/Button';
import { CloseIcon, PlayIcon, SyncIcon } from '@/shared/ui/Icon';
import { classes } from '@/shared/ui/classes';
import { useNavigation } from '@/app/routes';
import { backTo, replacing, said, working } from '@/features/player/outcomes';
import { activeTrackOf, readableTracksOf, trackProblemOf } from '@/features/player/tracks';
import { useAlignment } from '@/features/player/useAlignment';
import { useSync } from '@/features/player/useSync';
import { useFilmPalette } from './accent';
import { ComposedCover } from './ComposedCover';
import { fileFactsOf } from './facts';
import { useFrames } from './frames';
import { remainingOf } from './remaining';
import { resolutionOf, runtimeOf } from './runtime';
import { useLibrary } from './useLibrary';
import { useSheet } from './useSheet';
import { frameId } from './transition';
import styles from './FilmSheet.module.css';

/**
 * A film's page, which is where Subtext shows what it read.
 *
 * Everything on it was written when the film was scanned, so opening it reads
 * nothing from disk and costs a lookup in the library already in memory. That
 * is the whole reason the technical block can be this dense: none of it is
 * fetched, and none of it is guessed at either. A fact the file did not state
 * is left out.
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

  // The same picture the tile behind is showing: the frame from where the film
  // was stopped if one has been taken, and the poster otherwise.
  const frame = useFrames((held) => held.frames[film.id]?.url);
  const still = frame ?? (film.posterPath === null ? null : sourceOf(film.posterPath));
  const carryingOn = film.position !== null;

  const play = () => {
    onClose();
    openFilm(film.id);
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
        role="dialog"
        aria-modal="true"
        aria-labelledby={`film-sheet-${String(film.id)}`}
        tabIndex={-1}
        ref={panel}
        onKeyDown={onKeyDown}
      >
        <div className={styles.scroll}>
          <div className={styles.top}>
            <span className={styles.art} aria-hidden="true">
              {still !== null && <img className={styles.still} src={still} alt="" />}
            </span>

            <button type="button" className={styles.close} aria-label="Close" onClick={onClose}>
              <CloseIcon size={12} />
            </button>

            <motion.span layoutId={frameId(film.id)} className={styles.cover}>
              {still === null ? (
                <ComposedCover film={film} />
              ) : (
                <img className={styles.still} src={still} alt="" draggable={false} />
              )}
            </motion.span>

            <div className={styles.head}>
              <h2 className={styles.title} id={`film-sheet-${String(film.id)}`}>
                {film.title}
              </h2>
              <p className={styles.sub}>{headlineOf(film)}</p>

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
            </div>
          </div>

          <div className={styles.body}>
            <section className={styles.block}>
              <h3 className={styles.heading}>The file</h3>
              <dl className={styles.facts}>
                {fileFactsOf(film).map((row) => (
                  <div className={styles.fact} key={row.label}>
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
            </section>

            <section className={styles.block}>
              <h3 className={styles.heading}>Sound</h3>
              {film.details === null || film.details.audio.length === 0 ? (
                <p className={styles.none}>Nothing recorded</p>
              ) : (
                film.details.audio.map((audio, at) => (
                  <Row key={`${audio.codec}-${String(at)}`} language={audio.language}>
                    <span className={styles.what}>{soundOf(audio)}</span>
                  </Row>
                ))
              )}

              <h3 className={classes(styles.heading, styles.second)}>Subtitles</h3>
              {film.tracks.length === 0 ? (
                <p className={styles.none}>None found</p>
              ) : (
                film.tracks.map((track) => (
                  <Row key={track.id} language={track.language}>
                    <span className={styles.what}>{subtitleOf(track)}</span>
                    {tagsOf(track).map((tag) => (
                      <span key={tag.name} className={classes(styles.tag, tag.warn && styles.warn)}>
                        {tag.name}
                      </span>
                    ))}
                  </Row>
                ))
              )}

              <AlignOffer film={film} />
            </section>
          </div>
        </div>
      </div>
    </div>
  );
}

/** One track, sound or subtitle, with what it says it is in front of it. */
function Row({ language, children }: { language: string | null; children: ReactNode }) {
  return (
    <div className={styles.track}>
      <span className={styles.lang}>{language ?? '—'}</span>
      {children}
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
function AlignOffer({ film }: { film: FilmView }) {
  const preferred = useSetting('subtitleLanguage');
  const readable = readableTracksOf(film.tracks);
  // The track it would be watched with, or the first that could be read where
  // subtitles have been turned off for this film.
  const track = activeTrackOf(film, preferred) ?? readable[0] ?? null;

  const sync = useSync(track);
  const alignment = useAlignment(track, sync);
  const { state } = alignment;

  // A film with only pictures for subtitles has nothing to measure, and saying
  // so is the subtitle row's job rather than an offer that would decline.
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
    return (
      <div className={styles.align}>
        <p className={styles.offerTitle}>{title}</p>
        <p className={styles.offer} role="status">
          {sentence}
        </p>
        <div className={styles.row}>
          {state.outcome.outcome === 'aligned' && !state.undone && (
            <Button onClick={alignment.undo}>
              Put it back {backTo(state.outcome.previous.offsetMs)}
            </Button>
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
        match. It takes a few seconds, and the film need not be open.
      </p>
      <div className={styles.row}>
        <Button onClick={alignment.start}>Listen and line up</Button>
      </div>
    </div>
  );
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

/**
 * The line under the title: what somebody choosing a film would say about it.
 *
 * The shorthand rather than the figures, since the block below carries the
 * exact ones. A fact that is not known is left out, so a film nobody has opened
 * and whose container was never read is its year alone.
 */
function headlineOf(film: FilmView): string {
  const video = film.details?.video;
  const picture = [resolutionOf(film), video?.codec].filter((part) => part != null).join(' ');

  return [film.year === null ? null : String(film.year), runtimeOf(film.durationMs), picture]
    .filter((part) => part !== null && part !== '')
    .join(' · ');
}

/** How far through the film somebody is, as a whole number. */
function watchedPercent(film: FilmView): number {
  const position = film.position;
  if (position === null) return 0;
  if (position.finished) return 100;
  return Math.round((position.progress ?? 0) * 100);
}

/** What a sound track is, which is its codec and how many speakers it is for. */
function soundOf(audio: AudioView): string {
  return [audio.codec, audio.layout].filter((part) => part !== null).join(' ');
}

/** Where a subtitle track came from, and how much of it there is. */
function subtitleOf(track: TrackView): string {
  const where = track.origin === 'stream' ? 'Inside the film' : 'Beside the film';
  if (track.cueCount === 0) return where;
  return `${where} · ${track.cueCount.toLocaleString('en-GB')} lines`;
}

/**
 * What else is worth saying about a track.
 *
 * A track of pictures is named and marked as one Subtext cannot read, in the
 * words the README uses about them. It is still listed: a film whose only
 * subtitles are pictures should say so rather than appear to have none.
 */
function tagsOf(track: TrackView): { name: string; warn?: boolean }[] {
  const problem = trackProblemOf(track);

  return [
    track.forced ? { name: 'Forced' } : null,
    track.hearingImpaired ? { name: 'For the hard of hearing' } : null,
    problem === null ? null : { name: problem, warn: true },
  ].filter((tag) => tag !== null);
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
