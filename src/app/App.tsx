import { useCallback, useEffect, useState } from 'react';
import {
  commands,
  events,
  type FilmView,
  type FolderView,
  type ScanProgressed,
} from '../shared/ipc/bindings';

/**
 * A plain window over the scanner, so that pointing the application at a real
 * folder can be tried without a design in front of it. It shows what was found,
 * what was paired, and how far a scan has got, and nothing else. The library
 * screen replaces it.
 */
export function App() {
  const [folders, setFolders] = useState<FolderView[]>([]);
  const [films, setFilms] = useState<FilmView[]>([]);
  const [progress, setProgress] = useState<ScanProgressed | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const show = useCallback(([listed, found]: [FolderView[], FilmView[]]) => {
    setFolders(listed);
    setFilms(found);
    setProblem(null);
  }, []);

  const complain = useCallback((failure: unknown) => {
    setProblem(said(failure));
  }, []);

  // Reading the library is the one thing this window does that is not started
  // by a person, so it is the one place the state comes from outside React.
  const refresh = useCallback(() => {
    Promise.all([expect(commands.listFolders()), expect(commands.listLibrary())]).then(
      show,
      complain,
    );
  }, [show, complain]);

  useEffect(refresh, [refresh]);

  useEffect(() => {
    const listening = [
      events.scanProgressed.listen((event) => {
        setProgress(event.payload);
      }),
      // A scan that has finished has changed what there is to show.
      events.scanFinished.listen(refresh),
      events.scanFailed.listen((event) => {
        setProblem(event.payload.message);
      }),
    ];

    return () => {
      for (const pending of listening) {
        void pending.then((stop) => {
          stop();
        });
      }
    };
  }, [refresh]);

  const addFolder = async () => {
    try {
      const chosen = await expect(commands.chooseFolder());
      if (chosen === null) return;
      await expect(commands.addFolder(chosen));
      refresh();
    } catch (failure) {
      setProblem(said(failure));
    }
  };

  const rescan = async () => {
    try {
      await expect(commands.rescan());
    } catch (failure) {
      setProblem(said(failure));
    }
  };

  return (
    <main>
      <h1>Subtext</h1>

      <p>
        <button
          type="button"
          onClick={() => {
            void addFolder();
          }}
        >
          Add a folder
        </button>{' '}
        <button
          type="button"
          onClick={() => {
            void rescan();
          }}
        >
          Rescan
        </button>
      </p>

      {problem !== null && <p role="alert">{problem}</p>}

      {progress !== null && (
        <p aria-live="polite">
          {progress.stage}: {progress.filmsFound} films, {progress.subtitlesRead} of{' '}
          {progress.subtitlesToRead} subtitle files read, {progress.cuesIndexed} lines indexed
        </p>
      )}

      <h2>Folders</h2>
      {folders.length === 0 ? (
        <p>No folders yet.</p>
      ) : (
        <ul>
          {folders.map((folder) => (
            <li key={folder.id}>
              {folder.path} ({folder.films} films
              {folder.watching ? ', watching' : ', not watching'})
            </li>
          ))}
        </ul>
      )}

      <h2>Films</h2>
      {films.length === 0 ? (
        <p>Nothing found yet.</p>
      ) : (
        <ul>
          {films.map((film) => (
            <li key={film.id}>
              {film.title}
              {film.year === null ? '' : ` (${String(film.year)})`}
              {`, ${String(lines(film))} lines`}
              {film.missing ? ', missing' : ''}
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}

/** How many lines of dialogue a film has across all of its subtitle tracks. */
function lines(film: FilmView): number {
  return film.tracks.reduce((total, track) => total + track.cueCount, 0);
}

/**
 * Turns a command's answer into its value, or throws what went wrong.
 *
 * The generated bindings hand back a result rather than throwing, which is the
 * right shape for a screen that wants to show the failure. This window only
 * wants the message.
 */
async function expect<T>(
  pending: Promise<{ status: 'ok'; data: T } | { status: 'error'; error: { message: string } }>,
): Promise<T> {
  const answer = await pending;
  if (answer.status === 'error') throw new Error(answer.error.message);
  return answer.data;
}

function said(failure: unknown): string {
  return failure instanceof Error ? failure.message : String(failure);
}
