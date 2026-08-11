import { useEffect, useRef } from 'react';
import { Button } from '@/shared/ui/Button';
import { CloseIcon, FolderIcon } from '@/shared/ui/Icon';
import { ipc, reasonFor } from '@/shared/ipc/client';
import { useImport } from '@/features/onboarding/useImport';
import { useLibrary } from '@/features/library/useLibrary';
import { Action, Choice } from './controls';
import { useSetting } from '@/shared/settings/useSettings';
import styles from './LibrarySection.module.css';

/**
 * The folders the library is a view onto, and what is made of what is in them.
 *
 * Changing how strictly names are paired starts a read of every folder, because
 * a preference about pairing that left the existing pairings alone would be a
 * preference that appeared to do nothing. A read of an unchanged folder is
 * almost free in every other case, which is what makes that affordable.
 */
export function LibrarySection() {
  const folders = useLibrary((library) => library.folders);
  const removeFolder = useLibrary((library) => library.removeFolder);
  const chooseFolder = useImport((state) => state.chooseFolder);
  const rebuild = useImport((state) => state.rebuild);
  const scanning = useImport((state) => state.progress !== null);

  useRereadWhenPairingChanges();

  return (
    <>
      <div className={styles.folders}>
        <div className={styles.heading}>
          <h3>Watched folders</h3>
          <Button onClick={() => void chooseFolder()}>
            <FolderIcon size={14} />
            Add a folder
          </Button>
        </div>

        {folders.length === 0 ? (
          <p className={styles.empty}>
            No folders are being watched. Films are read where they are: nothing is copied, moved or
            written beside them.
          </p>
        ) : (
          <ul className={styles.list}>
            {folders.map((folder) => (
              <li key={folder.id} className={styles.folder}>
                <span className={styles.glyph}>
                  <FolderIcon size={17} />
                </span>
                <span className={styles.path}>
                  <b data-selectable>{folder.path}</b>
                  <span>
                    {folder.watching ? (
                      <>
                        <i className={styles.live} />
                        watching
                      </>
                    ) : (
                      'not watching'
                    )}
                    {` · ${String(folder.films)} ${folder.films === 1 ? 'film' : 'films'}`}
                  </span>
                </span>
                <button
                  type="button"
                  className={styles.remove}
                  aria-label={`Stop watching ${folder.path}`}
                  onClick={() => void removeFolder(folder.id)}
                >
                  <CloseIcon size={15} />
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <Choice
        name="missingFilms"
        label="Films whose file has gone"
        hint="An unplugged drive takes its films with it. Either way Subtext keeps where you were up to and puts them back when the file returns."
        options={[
          { value: 'keep', label: 'Show them' },
          { value: 'hide', label: 'Hide them' },
        ]}
      />

      <Choice
        name="matching"
        label="Pairing subtitles with films"
        hint="Which subtitle belongs to which film is worked out from the names. Exact is for a collection named by hand; close enough also takes a subtitle whose name is the start of the film's."
        options={[
          { value: 'relaxed', label: 'Close enough' },
          { value: 'exact', label: 'Exact names' },
        ]}
      />

      <Action
        label="Rebuild the search index"
        hint="Builds the index again from the dialogue already read. For a library where search has stopped finding lines that are plainly there."
      >
        <Button onClick={() => void rebuild()} disabled={scanning}>
          {scanning ? 'Reading your films' : 'Rebuild'}
        </Button>
      </Action>
    </>
  );
}

/**
 * Reads every folder again when the pairing preference changes.
 *
 * Quietly, without the sheet an import puts up: nothing was asked for beyond
 * the setting itself, and what comes of it is a folder list that says something
 * different afterwards. The first render is not a change, so opening the screen
 * costs nothing.
 */
function useRereadWhenPairingChanges() {
  const matching = useSetting('matching');
  const failed = useImport((state) => state.failed);
  const paired = useRef(matching);

  useEffect(() => {
    if (matching === paired.current) return;
    paired.current = matching;

    // Quiet while it works, but not quiet when it will not start. The setting
    // has already moved, and a read that never happened would leave the folder
    // list describing pairings made under the old answer.
    ipc.rescan().catch((failure: unknown) => {
      failed(
        `The setting was kept, but your folders could not be read again: ${reasonFor(failure)}`,
      );
    });
  }, [matching, failed]);
}
