import { type KeyboardEvent, useId } from 'react';
import type { TrackView } from '@/shared/ipc/bindings';
import { classes } from '@/shared/ui/classes';
import { trackNameOf, trackNoteOf } from './tracks';
import type { TrackChoice } from './useTrack';
import styles from './TrackMenu.module.css';

/**
 * Which subtitle the film is read with.
 *
 * A list of what the pairing found, said in terms of what each track is rather
 * than of the file it came from, with turning them off as one of the answers.
 * Radio buttons because that is what this is: a set of alternatives, one of
 * which is in force, which the arrow keys already move between without any of
 * it being written here.
 */

interface TrackMenuProps {
  choice: TrackChoice;
  onClose: () => void;
}

/** What the off row is worth as a value, since it is not a track. */
const NONE = 'none';

export function TrackMenu({ choice, onClose }: TrackMenuProps) {
  const group = useId();

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'Escape') return;
    // Taken here so that the shell does not also read it as leaving the film.
    event.preventDefault();
    onClose();
  };

  const chosen = choice.off ? NONE : String(choice.active?.id ?? NONE);

  return (
    <div className={styles.panel} onKeyDown={onKeyDown}>
      <fieldset className={styles.set}>
        <legend className={styles.title}>Subtitles</legend>

        {choice.tracks.map((track) => (
          <Row
            key={track.id}
            group={group}
            value={String(track.id)}
            chosen={chosen}
            name={trackNameOf(track)}
            note={noteFor(track)}
            onChoose={() => {
              choice.choose(track);
            }}
          />
        ))}

        <Row
          group={group}
          value={NONE}
          chosen={chosen}
          name="No subtitles"
          note={null}
          onChoose={() => {
            choice.choose(null);
          }}
        />
      </fieldset>
    </div>
  );
}

/** What a row says about a track under its name. */
function noteFor(track: TrackView): string | null {
  const note = trackNoteOf(track);
  const lines = `${track.cueCount.toLocaleString()} lines`;
  return note === null ? lines : `${note} · ${lines}`;
}

interface RowProps {
  group: string;
  value: string;
  chosen: string;
  name: string;
  note: string | null;
  onChoose: () => void;
}

function Row({ group, value, chosen, name, note, onChoose }: RowProps) {
  return (
    <label className={classes(styles.row, value === chosen && styles.on)}>
      {/*
       * Named here rather than left to the two lines below it, which are laid
       * out as a grid and would be read as one run-on word.
       */}
      <input
        type="radio"
        name={group}
        className={styles.hidden}
        checked={value === chosen}
        onChange={onChoose}
        aria-label={note === null ? name : `${name}, ${note}`}
      />
      <span className={styles.name}>{name}</span>
      {note !== null && <span className={styles.note}>{note}</span>}
    </label>
  );
}
