import { useEffect, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { CueView } from '@/shared/ipc/bindings';
import { clockOf } from '@/shared/media/clock';
import { NONE } from '@/shared/media/cues';
import { CloseIcon } from '@/shared/ui/Icon';
import { classes } from '@/shared/ui/classes';
import { estimateHeight, inView } from './rows';
import styles from './Transcript.module.css';

/**
 * The dialogue, beside the film.
 *
 * The lines are already parsed, corrected and sorted, and they are already in
 * memory because stepping between them needs them there. This panel reads that
 * same array and copies none of it: what is drawn over the picture and what is
 * listed here are the same cues, so a line cannot be on screen and missing from
 * the list beside it.
 *
 * Virtualised for the same reason the wall of covers is. A two hour film is
 * five thousand lines, and five thousand rows in the document is a panel that
 * cannot be scrolled while a film decodes beside it. Only what is in view
 * exists, and the rest is arithmetic.
 *
 * Nothing here runs in the frame loop. The mark moves when the line changes,
 * which over a film is a few hundred times rather than a few hundred thousand.
 */

interface TranscriptProps {
  cues: readonly CueView[];
  /** The line being spoken, or [`NONE`] during a silence. */
  active: number;
  onSeek: (ms: number) => void;
  onClose: () => void;
}

/** How many rows either side of the view are kept ready. */
const OVERSCAN = 6;

/**
 * How far past the top of the panel a line is put when the panel follows.
 *
 * Not the middle: what has just been said is worth less than what is about to
 * be, so the line sits high and leaves the room below it to read ahead.
 */
const FOCUS = 0.38;

export function Transcript({ cues, active, onSeek, onClose }: TranscriptProps) {
  const scroller = useRef<HTMLDivElement>(null);

  // The virtualiser hands back answers that change as the panel is scrolled, so
  // a remembered component holding it would draw the rows it had when it was
  // last let go. This component redraws as it is scrolled, on purpose.
  // eslint-disable-next-line react-hooks/incompatible-library -- see above
  const virtualiser = useVirtualizer({
    count: cues.length,
    getScrollElement: () => scroller.current,
    estimateSize: (index) => estimateHeight(cues[index]?.text ?? ''),
    measureElement: measured,
    overscan: OVERSCAN,
    // Something to work from before the panel has been laid out, so that the
    // first paint has lines in it rather than an empty column.
    initialRect: { width: 340, height: window.innerHeight },
  });

  // The line the film is up to, which is the last one spoken rather than the
  // one being spoken. Most of a film is silence, and a panel that unmarked
  // itself between every pair of lines would say nothing about where the film
  // had got to for most of its length.
  const spoken = useRef(NONE);
  if (active !== NONE) spoken.current = active;
  const at = spoken.current;

  /*
   * Keeping the film's place in view, without taking the panel away from a
   * reader. The decision is [`inView`]'s and the reasoning is written there;
   * what is held here is the line that was marked before this one, since that
   * is the row whose position says whether the panel was still being watched or
   * had been scrolled away from.
   *
   * The virtualiser is read when a line changes rather than depended on: it is
   * a fresh object on every render, and depending on it would scroll the list
   * on every render instead of on every line.
   */
  const latest = useRef(virtualiser);
  useEffect(() => {
    latest.current = virtualiser;
  });

  const was = useRef(NONE);
  useEffect(() => {
    const box = scroller.current;
    if (at === NONE || box === null) return;

    const list = latest.current;
    const before = was.current;
    was.current = at;

    // Nothing marked before this, which is the panel being opened part way
    // through a film. Its place is where the film is, whatever was on screen.
    if (before !== NONE && !onScreen(list, before)) return;

    const top = topOf(list, at);
    if (top === undefined) return;

    // Set rather than asked for, because where the line goes is this panel's
    // decision and not the virtualiser's, and because the element clamps a
    // position past the end of its own content without being told to.
    box.scrollTop = Math.max(0, top - box.clientHeight * FOCUS);
  }, [at]);

  return (
    <aside className={styles.panel} aria-label="Dialogue">
      <header className={styles.top}>
        <h2 className={styles.heading}>Dialogue</h2>
        <p className={styles.count}>{`${cues.length.toLocaleString('en-GB')} lines`}</p>
        <button
          type="button"
          className={styles.close}
          onClick={onClose}
          aria-label="Hide the dialogue"
        >
          <CloseIcon size={12} />
        </button>
      </header>

      <div ref={scroller} className={styles.scroller}>
        <div className={styles.list} style={{ height: `${String(virtualiser.getTotalSize())}px` }}>
          {virtualiser.getVirtualItems().map((row) => {
            const cue = cues[row.index];
            if (cue === undefined) return null;

            return (
              <button
                key={row.key}
                type="button"
                ref={virtualiser.measureElement}
                data-index={row.index}
                className={classes(styles.line, row.index === at && styles.current)}
                // What the film has already been through, which is the whole of
                // the difference between the lines either side of the mark.
                // Set as an attribute rather than as a style so that a line
                // changing costs one attribute on the two rows that changed.
                data-said={row.index < at ? '' : undefined}
                style={{ transform: `translateY(${String(row.start)}px)` }}
                aria-current={row.index === at ? 'true' : undefined}
                onClick={() => {
                  onSeek(cue.startMs);
                }}
              >
                <span className={styles.at}>{clockOf(cue.startMs)}</span>
                <span className={styles.said}>{cue.text}</span>
              </button>
            );
          })}
        </div>
      </div>
    </aside>
  );
}

/** What the virtualiser is asked for, which is less than all of it. */
type Virtualiser = ReturnType<typeof useVirtualizer<HTMLDivElement, Element>>;

/** Whether a row can be seen, in the numbers the virtualiser already holds. */
function onScreen(list: Virtualiser, index: number): boolean {
  const top = topOf(list, index);
  // A row the virtualiser cannot place is not evidence of anything, and the
  // panel carries on following rather than stopping on a measurement it does
  // not have.
  if (top === undefined) return true;

  // Both stand at nothing until the panel has been laid out and scrolled, and
  // nothing is the truth in both cases: the list is at its top, and a panel of
  // no height has nothing in front of anybody.
  return inView(top, list.scrollOffset ?? 0, list.scrollRect?.height ?? 0);
}

/** Where a row begins down the list, once the virtualiser has placed it. */
function topOf(list: Virtualiser, index: number): number | undefined {
  return list.measurementsCache[index]?.start;
}

/**
 * How tall a row turned out to be.
 *
 * The platform is asked, and where it has nothing to say the estimate stands.
 * A row is measured the moment it is drawn, but a panel that has not been laid
 * out yet reports every row as nothing at all, and rows of no height would
 * collapse the list into a single point.
 */
function measured(element: Element): number {
  const height = element.getBoundingClientRect().height;
  if (height > 0) return height;

  return estimateHeight(element.textContent);
}
