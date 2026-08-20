import { useState } from 'react';
import type { CoversTaken } from '@/shared/ipc/bindings';
import { ipc, reasonFor } from '@/shared/ipc/client';
import { Button } from '@/shared/ui/Button';
import { Sheet } from '@/shared/ui/Sheet';
import { CoverReport } from './CoverReport';
import { useLibrary } from './useLibrary';
import styles from './PosterFolder.module.css';

/**
 * Dressing a whole library from one folder of pictures.
 *
 * Somebody who has a folder of posters has already done the work of collecting
 * them, and matching them to films by name is the only part left. It is one
 * action rather than a screen: point at the folder, and the report says what
 * came of it.
 *
 * Nothing is written anywhere near anybody's films. The pictures stay where
 * they are and the library records where to find them, which is what choosing
 * a cover one at a time does as well.
 */
export function PosterFolder() {
  const films = useLibrary((library) => library.films);
  const refresh = useLibrary((library) => library.refresh);
  const [busy, setBusy] = useState(false);
  const [taken, setTaken] = useState<CoversTaken | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const choose = async () => {
    setProblem(null);
    try {
      const folder = await ipc.chooseFolder();
      if (folder === null) return;

      setBusy(true);
      const covered = await ipc.coversFromFolder(folder);
      // The library is read again before the report is shown, so what it counts
      // is what the rows now say rather than what this call hoped they would.
      await refresh();
      setTaken(covered);
    } catch (failure) {
      setProblem(reasonFor(failure));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <span className={styles.action}>
        <Button disabled={busy} onClick={() => void choose()}>
          {busy ? 'Matching them up' : 'Choose a folder'}
        </Button>
        {problem !== null && (
          <span role="alert" className={styles.problem}>
            {problem}
          </span>
        )}
      </span>

      <Sheet
        open={taken !== null}
        title="Where your covers came from"
        description={taken === null ? undefined : matched(taken)}
        footer={
          <Button
            tone="primary"
            onClick={() => {
              setTaken(null);
            }}
          >
            Done
          </Button>
        }
        onClose={() => {
          setTaken(null);
        }}
      >
        <CoverReport films={films} />
      </Sheet>
    </>
  );
}

/** What the folder itself accounted for, above the tally of the whole library. */
function matched({ matched: covered, unmatched }: CoversTaken): string {
  const took = `${String(covered)} ${covered === 1 ? 'film' : 'films'} took a cover from that folder`;
  return unmatched === 0 ? `${took}.` : `${took}. The other ${String(unmatched)} are as they were.`;
}
