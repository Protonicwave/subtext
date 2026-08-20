import { type KeyboardEvent, useState } from 'react';
import { classes } from '@/shared/ui/classes';
import { CHECK, backTo, replacing, said, working } from './outcomes';
import type { Alignment } from './useAlignment';
import type { Check } from './useCheck';
import { RATES, STEP_MS, type Sync, asWritten, offsetLabel, sameRate } from './useSync';
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
  /** Working the offset out by listening to the film rather than by ear. */
  alignment: Alignment;
  /** Watching what it measured, rather than reading the figures for it. */
  check: Check;
  onClose: () => void;
}

export function SyncPanel({ sync, alignment, check, onClose }: SyncPanelProps) {
  const named = RATES.find((known) => sameRate(known.value, sync.rate));
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

      <AlignAction alignment={alignment} check={check} offsetMs={sync.offsetMs} />

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

/**
 * Working the offset out by listening to the film, rather than by ear.
 *
 * The same number arrived at a different way, so it sits under the nudge
 * buttons rather than somewhere of its own. Everything it can say is a
 * sentence: a refusal here means the file cannot be measured, which is worth
 * knowing and is not a fault to be reported in red.
 */
function AlignAction({
  alignment,
  check,
  offsetMs,
}: {
  alignment: Alignment;
  check: Check;
  offsetMs: number;
}) {
  const { state } = alignment;

  if (state.phase === 'confirming') {
    return (
      <div className={styles.aligning}>
        <p className={styles.note}>{replacing(offsetMs)}</p>
        <div className={styles.row}>
          <button type="button" className={styles.nudge} onClick={alignment.confirm}>
            Measure anyway
          </button>
          <button
            type="button"
            className={classes(styles.nudge, styles.quiet)}
            onClick={alignment.dismiss}
          >
            Keep mine
          </button>
        </div>
      </div>
    );
  }

  if (state.phase === 'running') {
    return (
      <div className={styles.aligning}>
        <div className={styles.row}>
          <p className={styles.working} aria-live="polite">
            {working(state.stage)}
          </p>
          <button
            type="button"
            className={classes(styles.nudge, styles.quiet, styles.stop)}
            onClick={alignment.cancel}
          >
            Stop
          </button>
        </div>
        <div
          className={styles.progress}
          role="progressbar"
          aria-label="Listening to the film"
          aria-valuenow={Math.round(state.fraction * 100)}
        >
          <div className={styles.done} style={{ scale: `${String(state.fraction)} 1` }} />
        </div>
      </div>
    );
  }

  if (state.phase === 'outcome') {
    const { title, sentence } = said(state.outcome);
    // The measurement that is in force and could still be taken back, which is
    // the only one there is anything to watch or to undo.
    const written = state.outcome.outcome === 'aligned' && !state.undone ? state.outcome : null;

    return (
      <div className={styles.aligning}>
        <div className={styles.row}>
          <p className={styles.title}>{title}</p>
          <button
            type="button"
            className={classes(styles.nudge, styles.quiet, styles.stop)}
            onClick={alignment.dismiss}
          >
            Dismiss
          </button>
        </div>
        <p className={styles.note} role="status">
          {sentence}
        </p>

        {written !== null &&
          (check.watching ? (
            <div className={styles.answer}>
              <p className={styles.note}>{CHECK.ask}</p>
              <div className={styles.row}>
                <button type="button" className={styles.nudge} onClick={check.keep}>
                  {CHECK.keep}
                </button>
                <button
                  type="button"
                  className={classes(styles.nudge, styles.quiet)}
                  onClick={check.putBack}
                >
                  Put it back {backTo(written.previous.offsetMs)}
                </button>
              </div>
            </div>
          ) : (
            <div className={styles.answer}>
              <div className={styles.row}>
                {check.offered && (
                  <button type="button" className={styles.nudge} onClick={check.start}>
                    {CHECK.offer}
                  </button>
                )}
                <button
                  type="button"
                  className={classes(styles.nudge, styles.quiet)}
                  onClick={alignment.undo}
                >
                  Put it back {backTo(written.previous.offsetMs)}
                </button>
              </div>
              {check.offered && <p className={styles.note}>{CHECK.note}</p>}
            </div>
          ))}
      </div>
    );
  }

  return (
    <div className={styles.aligning}>
      <button
        type="button"
        className={styles.nudge}
        onClick={alignment.start}
        disabled={!alignment.available}
      >
        Listen and line up
      </button>
      <p className={styles.note}>
        Reads the film&rsquo;s audio to find where the talking is, and moves the subtitles to match.
        It takes a few seconds and the film keeps playing. The A key does the same.
      </p>
    </div>
  );
}
