/**
 * How tall a line of dialogue is before anything has been drawn.
 *
 * The panel is virtualised, so the height of the whole list and the position of
 * every row below the view are worked out from a guess until a row has been on
 * screen once and measured. A guess that is roughly right means the scrollbar
 * barely moves under the thumb as the film plays; one fixed number for lines
 * that run to three of them means it jumps.
 *
 * This is arithmetic over the text and nothing else, which is what lets the
 * virtualiser ask it about five thousand lines without touching the document.
 */

/** The height of one drawn row of text, at the panel's reading size. */
const LINE = 22;

/** The space above and below the text of one cue. */
const AROUND = 16;

/**
 * Roughly how many characters fit across the panel.
 *
 * Taken from the panel's width at the middle of the range it is drawn across.
 * It is only ever a guess: the measurement replaces it the moment the row is
 * drawn.
 */
const ACROSS = 38;

export function estimateHeight(text: string): number {
  let drawn = 0;
  // A cue broken over two lines in the file keeps its break in the panel, so
  // each of them is wrapped on its own rather than the cue being one run.
  for (const line of text.split('\n')) {
    drawn += Math.max(1, Math.ceil(line.length / ACROSS));
  }
  return Math.max(1, drawn) * LINE + AROUND;
}

/**
 * Whether a row beginning at `top` can be seen.
 *
 * This is what decides whether the panel follows the film. The rule is that it
 * follows while the line it last marked is still in front of the reader, and
 * stops the moment it is not: somebody who has scrolled back four minutes to
 * find a line has taken the panel away from the film on purpose, and dragging
 * them forward again on the next thing said would be worse than never
 * following at all. Scrolling back to where the film is up to starts it again,
 * which is the same rule read the other way round and needs nothing to be
 * pressed.
 *
 * Only the top of the row is tested. A line taller than the panel is
 * pathological, and a row whose start has passed out of view has been read.
 */
export function inView(top: number, offset: number, height: number): boolean {
  return top >= offset && top <= offset + height;
}
