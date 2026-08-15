import { type RefObject, useEffect, useState } from 'react';
import { NONE, type Timeline, activeAt } from '@/shared/media/cues';

/**
 * The line of dialogue on screen, kept in step with the picture.
 *
 * Driven by the frame loop rather than by the element's own `timeupdate`, which
 * fires about four times a second. A quarter of a second is a quarter of a
 * second late on every line, and the plan is for subtitle timing to match the
 * file within a frame.
 *
 * The loop is written to do nothing most of the time. It reads the position,
 * runs a binary search, and returns; state is only set when the line actually
 * changes, which over a film is a few hundred times rather than a few hundred
 * thousand. Nothing in it allocates.
 *
 * What comes back is the index rather than the cue, because stepping by line
 * needs to know which line this is among the others and not only what it says.
 */
export function useActiveLine(
  video: RefObject<HTMLVideoElement | null>,
  timeline: Timeline,
): number {
  // What was found, and what it was found in. Held together so that dialogue
  // arriving, or a second film being opened, cannot leave the last film's line
  // on screen for the frame before the loop next runs.
  const [found, setFound] = useState<{ timeline: Timeline; index: number }>({
    timeline,
    index: NONE,
  });

  useEffect(() => {
    let shown = NONE;
    let frame = 0;

    const tick = () => {
      const element = video.current;
      if (element !== null) {
        const at = activeAt(timeline, element.currentTime * 1000);
        if (at !== shown) {
          shown = at;
          setFound({ timeline, index: at });
        }
      }
      frame = requestAnimationFrame(tick);
    };

    frame = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [timeline, video]);

  return found.timeline === timeline ? found.index : NONE;
}
