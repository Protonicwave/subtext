import {
  type ClipboardEvent,
  type CSSProperties,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useRef,
} from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { CueView } from '@/shared/ipc/bindings';
import { clockOf } from '@/shared/media/clock';
import { NONE } from '@/shared/media/cues';
import { CloseIcon } from '@/shared/ui/Icon';
import { classes } from '@/shared/ui/classes';
import { passageOf, selectedIn } from './passage';
import { estimateHeight } from './rows';
import { useFollow } from './useFollow';
import styles from './TranscriptPanel.module.css';

/**
 * The dialogue, beside the film.
 *
 * This is the panel the whole architecture exists for: the subtitles are not
 * pixels over the picture but a list that can be read, followed, clicked and
 * copied out of.
 *
 * Virtualised for the same reason the poster grid is. A two hour film is five
 * thousand lines, and five thousand rows in the document is a panel that cannot
 * be scrolled at sixty frames a second with a film decoding beside it. Only
 * what is in view exists, and the rest is arithmetic.
 *
 * The list is scrolled by setting its position rather than by asking the
 * platform to animate it, because the easing belongs in the stylesheet with the
 * rest of the motion, where the system's own preference for less of it is
 * already honoured.
 */

interface TranscriptPanelProps {
  cues: readonly CueView[];
  /** The line being spoken, or [`NONE`] during a silence. */
  active: number;
  onSeek: (ms: number) => void;
  onClose: () => void;
}

/** How many rows either side of the view are kept ready. */
const OVERSCAN = 6;

/**
 * How far past the top of the panel the line being spoken sits.
 *
 * Not the middle: what has just been said is worth less than what is about to
 * be, so the line sits high and leaves the room below it to read ahead.
 */
const FOCUS = 0.38;

/** The keys that move the list rather than the film. */
const PAGING = new Set(['PageUp', 'PageDown', 'Home', 'End']);

export function TranscriptPanel({ cues, active, onSeek, onClose }: TranscriptPanelProps) {
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
    // Something to work from before the panel has been laid out, so the first
    // paint has lines in it rather than an empty column.
    initialRect: { width: 340, height: window.innerHeight },
  });

  // Read by the scrolling, which has to be a stable function: it is what the
  // following reacts on, and one made afresh each render would scroll the list
  // on every render rather than on every line.
  const latest = useRef(virtualiser);
  useEffect(() => {
    latest.current = virtualiser;
  });

  const goTo = useCallback((index: number) => {
    const element = scroller.current;
    if (element === null) return;

    const found = latest.current.getOffsetForIndex(index, 'start');
    if (found === undefined) return;

    element.scrollTop = Math.max(0, found[0] - element.clientHeight * FOCUS);
  }, []);

  // The line to keep in view, which is the last one spoken rather than the one
  // being spoken: most of a film is silence, and a panel that scrolled back to
  // the top of the film every time somebody stopped talking would be unusable.
  const spoken = useRef(NONE);
  if (active !== NONE) spoken.current = active;

  const { following, release, resume } = useFollow(spoken.current, goTo);

  const onCopy = (event: ClipboardEvent<HTMLDivElement>) => {
    const chosen = selectedIn(event.currentTarget, cues, window.getSelection());
    if (chosen.length === 0) return;

    event.clipboardData.setData('text/plain', passageOf(chosen));
    event.preventDefault();
  };

  return (
    <aside className={styles.panel} aria-label="Transcript">
      <header className={styles.top}>
        <h2 className={styles.heading}>Transcript</h2>
        <p className={styles.count}>
          {cues.length === 0 ? 'No dialogue' : `${cues.length.toLocaleString('en-GB')} lines`}
        </p>
        <button
          type="button"
          className={styles.close}
          onClick={onClose}
          aria-label="Hide the transcript"
        >
          <CloseIcon size={12} />
        </button>
      </header>

      {cues.length === 0 ? (
        <p className={styles.nothing}>
          No subtitle file was paired with this film, so there is nothing to follow. Attaching one
          gives it a transcript and puts its dialogue into search.
        </p>
      ) : (
        <div
          ref={scroller}
          className={styles.scroller}
          // Scrolling by hand is what stops the panel following, and these are
          // the ways a person does it. A scroll event would not do: the panel
          // fires those itself every time it moves to the next line.
          onWheel={release}
          onTouchMove={release}
          onPointerDown={(event) => {
            // The scrollbar is part of this element and nothing else is: a
            // press that names the element itself, rather than a line inside
            // it, landed on the bar. Choosing a line is not scrolling.
            if (event.target === event.currentTarget) release();
          }}
          onKeyDown={(event) => {
            if (PAGING.has(event.key)) release();
          }}
          onCopy={onCopy}
        >
          <div
            className={styles.list}
            style={{ height: `${String(virtualiser.getTotalSize())}px` }}
          >
            {virtualiser.getVirtualItems().map((row) => {
              const cue = cues[row.index];
              if (cue === undefined) return null;

              return (
                <div
                  key={row.key}
                  ref={virtualiser.measureElement}
                  data-index={row.index}
                  className={classes(styles.line, row.index === active && styles.current)}
                  style={
                    {
                      transform: `translateY(${String(row.start)}px)`,
                      // How far this line is from the one being spoken, which
                      // is what fades the ones around it away.
                      '--away': Math.min(6, Math.abs(row.index - active)),
                    } as CSSProperties
                  }
                  // Not a button, because a passage is selected by dragging
                  // across these and the text inside a button cannot be. The
                  // role and the keys are what a button would have brought.
                  role="button"
                  tabIndex={0}
                  aria-current={row.index === active ? 'true' : undefined}
                  onClick={() => {
                    onSeek(cue.startMs);
                  }}
                  onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
                    if (event.key !== 'Enter' && event.key !== ' ') return;
                    onSeek(cue.startMs);
                    // Space plays and pauses the film from anywhere, and a line
                    // that has been chosen with the keyboard has taken it.
                    event.preventDefault();
                  }}
                >
                  <span className={styles.at}>{clockOf(cue.startMs)}</span>
                  <span className={styles.said}>{cue.text}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {!following && cues.length > 0 && (
        <button type="button" className={styles.resume} onClick={resume}>
          Follow the film
        </button>
      )}
    </aside>
  );
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
