import { type KeyboardEvent, useState } from 'react';
import type { AlignStageView, AlignmentView } from '@/shared/ipc/bindings';
import { classes } from '@/shared/ui/classes';
import type { Alignment } from './useAlignment';
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
  /** Working the offset out by listening to the film rather than by ear. */
  alignment: Alignment;
  onClose: () => void;
}

/** Whether two rates are the same, allowing for the last bit of a division. */
function same(a: number, b: number): boolean {
  return Math.abs(a - b) < 1e-9;
}

export function SyncPanel({ sync, alignment, onClose }: SyncPanelProps) {
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

      <AlignAction alignment={alignment} offsetMs={sync.offsetMs} />

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
function AlignAction({ alignment, offsetMs }: { alignment: Alignment; offsetMs: number }) {
  const { state } = alignment;

  if (state.phase === 'confirming') {
    return (
      <div className={styles.aligning}>
        <p className={styles.note}>
          The subtitles are moved by {offsetLabel(offsetMs)} already. Listening to the film will
          replace that with what it measures.
        </p>
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
        {state.outcome.outcome === 'aligned' && !state.undone && (
          <button type="button" className={styles.nudge} onClick={alignment.undo}>
            Put it back {backTo(state.outcome.previous.offsetMs)}
          </button>
        )}
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

/**
 * Where undoing would put the timing back to.
 *
 * A film nobody had adjusted goes back to the file as written rather than back
 * to a value, and the readout says that in words, so the sentence is built
 * around it rather than around a number that is not there.
 */
function backTo(offsetMs: number): string {
  return offsetMs === 0 ? 'as written' : `to ${offsetLabel(offsetMs)}`;
}

/** What the reading is doing at the moment. */
function working(stage: AlignStageView): string {
  return stage === 'reading' ? 'Listening to the film' : 'Matching the dialogue';
}

/**
 * What to say about how it ended.
 *
 * Every ending in the typed outcome has a wording here, including the ones that
 * changed nothing, and each of those says that the bracket keys are still there.
 * Being told that a file cannot be measured is only useful alongside what to do
 * instead.
 */
function said(outcome: AlignmentView): { title: string; sentence: string } {
  switch (outcome.outcome) {
    case 'aligned':
      return {
        title: 'Lined up',
        sentence: `The subtitles now run ${offsetLabel(outcome.correction.offsetMs)}${rateSaid(
          outcome.correction.rate,
        )}.`,
      };
    case 'too-few-cues':
      return {
        title: 'Not enough to go on',
        sentence: `This subtitle has ${String(outcome.cues)} lines, and it takes about ${String(
          outcome.wanted,
        )} to tell one moment of a film from another. Subtitles for signs and foreign dialogue are usually this short. The bracket keys still work.`,
      };
    case 'no-audio':
      return {
        title: 'Nothing to listen to',
        sentence:
          'This film carries no soundtrack, so there is nothing to measure the subtitles against. The bracket keys still work.',
      };
    case 'unsupported':
      return {
        title: 'Audio Subtext cannot read',
        sentence: `${
          outcome.codec === null ? 'This soundtrack is' : `This soundtrack is ${outcome.codec},`
        } in a format Subtext has no decoder for, so there is nothing to measure against. The bracket keys still work.`,
      };
    case 'uncertain':
      return {
        title: 'Not sure enough to say',
        sentence:
          'The lines in this subtitle do not fall where the talking in this film does, whatever they are moved by. It may belong to a different film or a different cut. Nothing has been changed, and the bracket keys still work.',
      };
    case 'unreadable':
      return { title: 'This film could not be read', sentence: outcome.message };
    case 'stopped':
      return { title: 'Stopped', sentence: 'The subtitles have been left as they were.' };
  }
}

/** The stretch, where there is one, named as the list names it. */
function rateSaid(rate: number): string {
  if (asWritten({ offsetMs: 0, rate })) return '';
  const named = RATES.find((known) => same(known.value, rate));
  return `, stretched ${named?.name ?? `by ${rate.toFixed(4)}`}`;
}
