import { Button } from '@/shared/ui/Button';
import { CloseIcon, FolderIcon } from '@/shared/ui/Icon';
import { useImport } from '@/features/onboarding/useImport';
import { useLibrary } from '@/features/library/useLibrary';
import styles from './SettingsScreen.module.css';

/**
 * The folders the library is a view onto.
 *
 * The settings screen proper, with its left rail and everything it can change,
 * comes once there is something to change. What has to exist now is a way to
 * see what is being watched and to stop watching it, so that adding a folder is
 * not a decision without an undo.
 */
export function SettingsScreen() {
  const folders = useLibrary((library) => library.folders);
  const removeFolder = useLibrary((library) => library.removeFolder);
  const chooseFolder = useImport((state) => state.chooseFolder);

  return (
    <div className={styles.screen}>
      <header className={styles.top}>
        <h1 className={styles.title}>
          Watched folders
          <small>Films are read where they are. Nothing is copied or moved.</small>
        </h1>
        <Button onClick={() => void chooseFolder()}>
          <FolderIcon size={14} />
          Add a folder
        </Button>
      </header>

      {folders.length === 0 ? (
        <p className={styles.empty}>No folders are being watched.</p>
      ) : (
        <ul className={styles.folders}>
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
  );
}
