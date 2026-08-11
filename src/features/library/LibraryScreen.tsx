import { FolderIcon } from '@/shared/ui/Icon';
import { Button } from '@/shared/ui/Button';
import { useNavigation } from '@/app/routes';
import { useImport } from '@/features/onboarding/useImport';
import { fileNameOf, linesOf, useLibrary } from './useLibrary';
import styles from './LibraryScreen.module.css';

/**
 * What is in the library, as a list.
 *
 * The poster grid, the continue watching row and the transition into the player
 * are the next piece of work. This is the same information without any of that,
 * so that a folder can be added and the result looked at.
 */
export function LibraryScreen() {
  const films = useLibrary((library) => library.films);
  const folders = useLibrary((library) => library.folders);
  const chooseFolder = useImport((state) => state.chooseFolder);
  const go = useNavigation((navigation) => navigation.go);

  const lines = films.reduce((total, film) => total + linesOf(film), 0);

  return (
    <div className={styles.screen}>
      <header className={styles.top}>
        <h1 className={styles.title}>
          Your films
          <small>
            {films.length === 0
              ? 'Nothing yet'
              : `${count(films.length, 'film')} · ${lines.toLocaleString('en-GB')} lines of dialogue`}
          </small>
        </h1>
        <Button tone="primary" onClick={() => void chooseFolder()}>
          <FolderIcon size={14} />
          Choose folder
        </Button>
      </header>

      {films.length === 0 ? (
        <p className={styles.empty}>
          {folders.length === 0
            ? 'No folders are being watched yet.'
            : 'Nothing was found in the folders being watched.'}
        </p>
      ) : (
        <ul className={styles.films}>
          {films.map((film) => (
            <li key={film.id}>
              <button
                type="button"
                className={styles.film}
                onClick={() => {
                  go({ screen: 'player', filmId: film.id });
                }}
              >
                <span className={styles.name}>
                  {film.title}
                  {film.year !== null && <em>{film.year}</em>}
                </span>
                <span className={styles.detail} data-selectable>
                  {fileNameOf(film.path)}
                </span>
                <span className={styles.lines}>
                  {film.missing
                    ? 'Missing'
                    : film.tracks.length === 0
                      ? 'No subtitles'
                      : `${linesOf(film).toLocaleString('en-GB')} lines`}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function count(many: number, thing: string): string {
  return `${String(many)} ${thing}${many === 1 ? '' : 's'}`;
}
