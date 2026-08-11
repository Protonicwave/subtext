import { CheckIcon, FolderIcon } from '@/shared/ui/Icon';
import { useImport } from './useImport';
import styles from './FirstRun.module.css';

const PROMISES = ['MP4 and MKV', 'SRT paired by filename', 'Works offline'];

/**
 * The first thing anybody sees.
 *
 * It has one job beyond starting the import, which is to say plainly that
 * nothing is uploaded. A local application that reads a person's film
 * collection should answer that question before it is asked, and answer it in
 * the first paragraph rather than in a privacy policy.
 */
export function FirstRun() {
  const chooseFolder = useImport((state) => state.chooseFolder);

  return (
    <section className={styles.screen}>
      <div className={styles.panel}>
        <p className={styles.kicker}>Subtext</p>
        <h1 className={styles.heading}>
          Your films are already on this machine. <i>Point at them.</i>
        </h1>
        <p className={styles.blurb}>
          Nothing is uploaded and nothing leaves your computer. Subtext reads the folder, pairs each
          film with its subtitles and indexes every line of dialogue.
        </p>

        <button type="button" className={styles.target} onClick={() => void chooseFolder()}>
          <span className={styles.glyph}>
            <FolderIcon size={26} />
          </span>
          <b>Choose your films folder</b>
          <span>Subtext will watch it and pick up anything you add later</span>
        </button>

        <p className={styles.alternative}>or drop files anywhere in the window</p>

        <ul className={styles.promises}>
          {PROMISES.map((promise) => (
            <li key={promise}>
              <CheckIcon size={13} />
              {promise}
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
