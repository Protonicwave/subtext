/**
 * How tall a line of the transcript is before anything has been drawn.
 *
 * The list is virtualised, so the scrollbar and the position of every row below
 * the window are worked out from a guess until the row has been on screen once
 * and measured. A guess that is roughly right means the scrollbar barely moves
 * under the thumb as the film plays; a fixed guess for lines that run to three
 * of them means it jumps.
 *
 * This is arithmetic over the text and nothing else, which is what lets the
 * virtualiser ask it for five thousand lines without touching the document.
 */

/** The height of one drawn line of dialogue, at the panel's reading size. */
const LINE = 26;

/** The space above and below the text of one cue, and its rule. */
const AROUND = 22;

/**
 * Roughly how many characters fit across the panel.
 *
 * Taken from the panel's width at its middle size in the reading face. It is
 * only ever a guess: the measurement replaces it the moment the row is drawn.
 */
const ACROSS = 34;

export function estimateHeight(text: string): number {
  let drawn = 0;
  for (const line of text.split('\n')) {
    drawn += Math.max(1, Math.ceil(line.length / ACROSS));
  }
  return Math.max(1, drawn) * LINE + AROUND;
}
