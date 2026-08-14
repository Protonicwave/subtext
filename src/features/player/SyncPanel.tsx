import { type KeyboardEvent, useState } from 'react';
import { classes } from '@/shared/ui/classes';
import { RATES, STEP_MS, type Sync, asWritten, offsetLabel } from './useSync';
import styles from './SyncPanel.module.css';

/**
 * Putting a subtitle back in step with its film.
 *
 * The offset is felt for rather than known, so it is two buttons and a readout
 * and the keys do the same thing without the panel being open at all. The rate
 * is the opposite: it is one of a handful of ratios between the framerates
 * films are released at, so it is a list. A slider would offer a value nobody
 * could arrive at twice.
 */

interface SyncPanelProps {
  sync: Sync;
  onClose: () => void;
}

/** Whether two rates are the same, allowing for the last bit of a division. */
function same(a: number, b: number): boolean {
  return Math.abs(a - b) < 1e-9;
}

export function SyncPanel({ sync, onClose }: SyncPanelProps) {
  const named = RATES.find((known) => same(known.value, sync.rate));
  const [byHand, setByHand] = useState(named === undefined);
  const [typed, setTyped] = useState(String(sync.rate));

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'Escape') return;
    // Taken here so that the shell does not also read it as leaving the film.
    event.preventDefault();
    onClose();
  };

  return (
    <div className={styles.panel} role="group" aria-label="Subtitle timing" onKeyDown={onKeyDown}>
      <div className={styles.row}>
        <p className={styles.title}>Subtitle timing</p>
        <p className={styles.readout} aria-live="polite">
          {offsetLabel(sync.offsetMs)}
        </p>
      </div>

      <div className={styles.row}>
        <button
          type="button"
          className={styles.nudge}
          onClick={() => {
            sync.nudge(-STEP_MS);
          }}
        >
          Earlier
        </button>
        <button
          type="button"
          className={styles.nudge}
          onClick={() => {
            sync.nudge(STEP_MS);
          }}
        >
          Later
        </button>
        <button
          type="button"
          className={classes(styles.nudge, styles.quiet)}
          onClick={sync.reset}
          disabled={asWritten(sync)}
        >
          As written
        </button>
      </div>

      <p className={styles.note}>
        Nudge until the lines land with the voices. {STEP_MS} milliseconds a press, and the square
        bracket keys do the same while you watch.
      </p>

      <label className={styles.field}>
        <span className={styles.label}>Framerate</span>
        <select
          className={styles.select}
          value={byHand ? 'other' : String(named?.value ?? 1)}
          onChange={(event) => {
            if (event.target.value === 'other') {
              setByHand(true);
              return;
            }
            setByHand(false);
            sync.setRate(Number(event.target.value));
          }}
        >
          {RATES.map((rate) => (
            <option key={rate.name} value={String(rate.value)}>
              {rate.name}
            </option>
          ))}
          <option value="other">A value of my own</option>
        </select>
      </label>

      {byHand && (
        <label className={styles.field}>
          <span className={styles.label}>Rate</span>
          <input
            type="number"
            className={styles.select}
            min={0.5}
            max={2}
            step={0.0001}
            value={typed}
            onChange={(event) => {
              setTyped(event.target.value);
            }}
            onBlur={() => {
              const value = Number(typed);
              if (Number.isFinite(value) && value > 0) sync.setRate(value);
            }}
          />
        </label>
      )}
    </div>
  );
}
