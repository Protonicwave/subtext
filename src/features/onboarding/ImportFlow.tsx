import { useState } from 'react';
import type { FilmView, ScanProgressed, ScanSummary, TrackView } from '@/shared/ipc/bindings';
import { events } from '@/shared/ipc/bindings';
import { ipc, reasonFor } from '@/shared/ipc/client';
import { useIpcEvent } from '@/shared/ipc/useIpcEvent';
import { Button } from '@/shared/ui/Button';
import { classes } from '@/shared/ui/classes';
import { AlertIcon, CheckIcon, PairedIcon } from '@/shared/ui/Icon';
import { ProgressRing } from '@/shared/ui/ProgressRing';
import { Sheet } from '@/shared/ui/Sheet';
import { fileNameOf, filmsIn, useLibrary } from '@/features/library/useLibrary';
import { foldersIn, unpairedIn, useImport } from './useImport';
import styles from './ImportFlow.module.css';

/**
 * Adding films to the library, from the first event to the last row.
 *
 * The scan is watched here rather than in the shell, because the events only
 * mean anything to the screens that show them, and because this is the one
 * place that decides what a scan finishing should put in front of somebody.
 */
export function ImportFlow() {
  const stage = useImport((state) => state.stage);
  const progress = useImport((state) => state.progress);
  const summaries = useImport((state) => state.summaries);
  const dismiss = useImport((state) => state.dismiss);
  const saw = useImport((state) => state.saw);
  const finished = useImport((state) => state.finished);
  const failed = useImport((state) => state.failed);

  useIpcEvent(events.scanProgressed, saw);
  useIpcEvent(events.scanFinished, finished);
  useIpcEvent(events.scanFailed, (failure) => {
    failed(failure.message);
  });

  return (
    <>
      <Sheet
        open={stage === 'indexing'}
        title="Reading your films"
        description="Pairing each film with its subtitles and indexing every line"
        dismissible={false}
        onClose={dismiss}
      >
        <Indexing progress={progress} />
      </Sheet>

      <Sheet
        open={stage === 'results'}
        title="What was found"
        description={<Summary summaries={summaries} />}
        footer={
          <>
            <span className={styles.hint}>
              Files stay where they are. Nothing is copied or moved.
            </span>
            <Button tone="primary" onClick={dismiss}>
              Done
            </Button>
          </>
        }
        onClose={dismiss}
      >
        <Pairings summaries={summaries} />
      </Sheet>
    </>
  );
}

/** How far the scan has got, in its own words. */
function Indexing({ progress }: { progress: ScanProgressed | null }) {
  const read = progress?.subtitlesRead ?? 0;
  const toRead = progress?.subtitlesToRead ?? 0;

  return (
    <div className={styles.indexing}>
      <ProgressRing fraction={progress?.fractionRead ?? 0} label="Indexing your films" />
      <h3 className={styles.indexingTitle}>{headingFor(progress)}</h3>
      <p className={styles.counts}>
        {(progress?.cuesIndexed ?? 0).toLocaleString('en-GB')} lines indexed
        {toRead > 0 && ` · ${String(read)} of ${String(toRead)} subtitle files read`}
      </p>
      {progress !== null && progress.filmsFound > 0 && (
        <p className={styles.found}>
          {progress.filmsFound.toLocaleString('en-GB')} films found,{' '}
          {progress.filmsPaired.toLocaleString('en-GB')} paired
        </p>
      )}
    </div>
  );
}

function headingFor(progress: ScanProgressed | null): string {
  switch (progress?.stage) {
    case undefined:
    case 'discovering':
      return 'Looking through the folder';
    case 'pairing':
      return 'Working out which subtitle belongs to which film';
    case 'indexing':
      return 'Indexing the dialogue';
    case 'finished':
      return 'Finishing off';
  }
}

/** The line under the heading: what was found, in counts. */
function Summary({ summaries }: { summaries: ScanSummary[] }) {
  const films = useLibrary((library) => library.films);
  const found = filmsIn(films, foldersIn(summaries));
  const paired = found.filter((film) => film.tracks.length > 0).length;
  const unpaired = unpairedIn(summaries).length;

  return (
    <>
      <b>{found.length}</b> {found.length === 1 ? 'film' : 'films'} · <b>{paired}</b> paired with
      subtitles
      {paired < found.length && ` · ${String(found.length - paired)} without`}
      {unpaired > 0 &&
        ` · ${String(unpaired)} subtitle ${unpaired === 1 ? 'file belongs' : 'files belong'} to no film`}
    </>
  );
}

function Pairings({ summaries }: { summaries: ScanSummary[] }) {
  const films = useLibrary((library) => library.films);
  const found = filmsIn(films, foldersIn(summaries));

  if (found.length === 0) {
    return <p className={styles.nothing}>No films were found in there.</p>;
  }

  return (
    <ul>
      {found.map((film) => (
        <Pairing key={film.id} film={film} />
      ))}
    </ul>
  );
}

/**
 * One film and the subtitle it was paired with.
 *
 * The row is the point of the whole screen: it says which file was matched to
 * which, and how sure that was, so that a wrong pairing is visible rather than
 * discovered later in the middle of a film.
 */
function Pairing({ film }: { film: FilmView }) {
  const [attaching, setAttaching] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const refresh = useLibrary((library) => library.refresh);

  const track: TrackView | undefined = film.tracks[0];

  const attach = async () => {
    setProblem(null);
    setAttaching(true);
    try {
      const chosen = await ipc.chooseSubtitle();
      if (chosen !== null) {
        await ipc.attachSubtitle(film.id, chosen);
        await refresh();
      }
    } catch (failure) {
      setProblem(reasonFor(failure));
    } finally {
      setAttaching(false);
    }
  };

  return (
    <li className={classes(styles.pair, !track && styles.unpaired)}>
      <span className={styles.tick}>
        {track ? <CheckIcon size={12} /> : <AlertIcon size={12} />}
      </span>

      <span className={styles.about}>
        <span className={styles.file} title={film.path} data-selectable>
          {fileNameOf(film.path)}
        </span>
        <span className={styles.subtitle}>
          <PairedIcon size={11} />
          {track ? fileNameOf(track.path) : 'No subtitle file found beside it'}
        </span>
        {problem !== null && (
          <span role="alert" className={styles.problem}>
            {problem}
          </span>
        )}
      </span>

      {track ? (
        <span className={styles.badge}>{badgeFor(track)}</span>
      ) : (
        <button
          type="button"
          className={styles.attach}
          disabled={attaching}
          onClick={() => void attach()}
        >
          {attaching ? 'Choosing…' : 'Attach .srt'}
        </button>
      )}
    </li>
  );
}

function badgeFor(track: TrackView): string {
  switch (track.matchKind) {
    case 'exact':
      return 'Matched';
    case 'approximate':
      return 'Close match';
    case 'by-hand':
      return 'Attached';
  }
}
