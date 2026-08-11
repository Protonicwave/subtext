import type { CueView } from '@/shared/ipc/bindings';
import { clockOf } from '@/shared/media/clock';

/**
 * Taking dialogue out of the application.
 *
 * The point of copying out of a transcript rather than off a screenshot is that
 * what you paste says when it was said. A line without its moment is a quotation
 * nobody can find again, so the timecode goes with it and is not an option.
 */

/**
 * A passage, written out with a timecode against each line.
 *
 * A cue that was broken over two lines in the file is joined back into one
 * here. Those breaks are where the text was made to fit across a picture, and
 * they mean nothing once it is somewhere else.
 */
export function passageOf(lines: readonly CueView[]): string {
  return lines
    .map((line) => {
      const said = flatten(line.text);
      const at = `[${clockOf(line.startMs)}]`;
      return said === '' ? at : `${at} ${said}`;
    })
    .join('\n');
}

function flatten(text: string): string {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line !== '')
    .join(' ');
}

/**
 * The lines a selection touches, in the order they are spoken.
 *
 * Worked out by asking each drawn line whether the selection reaches it, rather
 * than by reading the selected text. The text of a selection has the timecodes
 * in it, is cut off part way through the first and last lines, and says nothing
 * about which cues those were.
 *
 * Only the lines that are drawn can be found this way, which is what
 * virtualising the list costs: a selection dragged past the end of the window
 * covers lines that are not in the document to be asked.
 */
export function selectedIn(
  root: HTMLElement,
  cues: readonly CueView[],
  selection: Selection | null,
): CueView[] {
  if (selection === null || selection.isCollapsed) return [];

  // Firefox is the only one that hands back more than one, and it does so for
  // a selection made across a table. Cheap enough to honour everywhere.
  const ranges = Array.from({ length: selection.rangeCount }, (_, at) => selection.getRangeAt(at));

  const chosen: CueView[] = [];
  for (const line of root.querySelectorAll<HTMLElement>('[data-index]')) {
    if (!ranges.some((range) => range.intersectsNode(line))) continue;

    const cue = cues[Number(line.dataset.index)];
    if (cue !== undefined) chosen.push(cue);
  }

  return chosen;
}
