import { useMemo, useRef } from 'react';
import type { FilmView } from '@/shared/ipc/bindings';
import { FolderIcon } from '@/shared/ui/Icon';
import { Button } from '@/shared/ui/Button';
import { useSetting } from '@/shared/settings/useSettings';
import { useNavigation } from '@/app/routes';
import { useImport } from '@/features/onboarding/useImport';
import { Billboard } from './Billboard';
import { ContinueWatching } from './ContinueWatching';
import { Shelf } from './Shelf';
import { Wall } from './Wall';
import { billboardOf, shelvesOf } from './shelves';
import { libraryTotalOf, sizeOf } from './size';
import { useCapture } from './useCapture';
import { useLibrary } from './useLibrary';
import styles from './LibraryScreen.module.css';

/**
 * The screen you land on.
 *
 * One film shown large above rows built from the folders somebody already made,
 * or the one wall for anybody who would rather have every film at once. Both
 * are drawn against the same scrolling, which this screen owns rather than
 * leaving to the shell: the wall is virtualised, and a virtualiser has to be
 * told which element is scrolled to know what is in view.
 */
export function LibraryScreen() {
  const held = useLibrary((library) => library.films);
  const folders = useLibrary((library) => library.folders);
  const stopped = useLibrary((library) => library.resumable);
  const loaded = useLibrary((library) => library.loaded);
  const chooseFolder = useImport((state) => state.chooseFolder);
  const openFilm = useNavigation((navigation) => navigation.openFilm);

  const scroller = useRef<HTMLDivElement>(null);

  // A film whose file has gone is still a film Subtext knows about, and where
  // it was left is kept whichever of these is chosen. All that is in question
  // is whether it is worth a tile while the drive it is on is unplugged.
  const showMissing = useSetting('missingFilms') === 'keep';
  const films = useMemo(
    () => (showMissing ? held : held.filter((film) => !film.missing)),
    [held, showMissing],
  );
  const resumable = useMemo(
    () => (showMissing ? stopped : stopped.filter((film) => !film.missing)),
    [stopped, showMissing],
  );

  const shelved = useSetting('libraryLayout') === 'shelves';
  const shelves = useMemo(
    () => (shelved ? shelvesOf(films, resumable) : []),
    [shelved, films, resumable],
  );
  const billboard = useMemo(() => billboardOf(films, resumable), [films, resumable]);

  // Nothing is captured until the library has been read, so that a first run
  // does not open a decoder before there is anything to point it at.
  useCapture(films.length > 0);

  // No moment named, so the film opens where it was left.
  const open = (film: FilmView) => {
    openFilm(film.id);
  };

  return (
    <div className={styles.screen} ref={scroller}>
      {billboard && <Billboard film={billboard} onOpen={open} />}

      <div className={styles.inner}>
        <div className={styles.tools}>
          {/*
           * Announced, because this is the one place the screen says whether it
           * is still reading, and somebody who cannot see the rows fill in has
           * nothing else to go on.
           */}
          <p className={styles.count} role="status">
            {summaryOf(loaded, films, shelves.length)}
          </p>
          <Button onClick={() => void chooseFolder()}>
            <FolderIcon size={14} />
            Choose folder
          </Button>
        </div>

        {/*
         * Nothing at all until the first read has come back. Every one of the
         * messages below is a statement about what is on disk, and none of them
         * is true yet: a library of a thousand films would be told for a moment
         * that no folders are being watched.
         */}
        {loaded &&
          (films.length === 0 ? (
            <p className={styles.empty}>{emptyBecause(folders.length, held.length)}</p>
          ) : shelved ? (
            shelves.map((shelf) => <Shelf key={shelf.key} shelf={shelf} onOpen={open} />)
          ) : (
            <>
              <ContinueWatching films={resumable} onOpen={open} />
              <Wall films={films} scroller={scroller} onOpen={open} />
            </>
          ))}
      </div>
    </div>
  );
}

/**
 * The line above the rows: still reading, read and empty, or what there is.
 *
 * The size is left out entirely unless every film has one, since a total that
 * quietly skipped the films this build has not looked at yet would be wrong
 * without saying so.
 */
function summaryOf(loaded: boolean, films: readonly FilmView[], shelves: number): string {
  if (!loaded) return 'Reading your films';
  if (films.length === 0) return 'Nothing yet';

  const total = libraryTotalOf(films);
  return [
    count(films.length, 'film'),
    shelves > 0 ? count(shelves, 'shelf', 'shelves') : null,
    total === null ? null : sizeOf(total),
  ]
    .filter((part) => part !== null)
    .join(' · ');
}

/**
 * Why the library is empty, which is three different situations and not one.
 *
 * The third is a library that is entirely missing its files, with the setting
 * that hides those turned on: an unplugged drive, and worth saying so rather
 * than reporting that nothing was found.
 */
function emptyBecause(folders: number, held: number): string {
  if (folders === 0) return 'No folders are being watched yet.';
  if (held > 0) {
    return 'Every film here is missing its file. Reconnect the drive they are on, or show them again from the settings screen.';
  }
  return 'Nothing was found in the folders being watched.';
}

function count(many: number, thing: string, plural = `${thing}s`): string {
  return `${String(many)} ${many === 1 ? thing : plural}`;
}
