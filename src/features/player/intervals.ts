/**
 * The intervals the player works to.
 *
 * Each of these was a control on the settings screen, and each is a number
 * again. They share a shape: a person asked to choose one has to guess what
 * they would prefer without a film in front of them, and almost everybody keeps
 * whatever was there. A settings screen made of questions like that reads as a
 * list of everything that could be configured rather than as a set of
 * decisions, so the decisions are made here and written down with them.
 *
 * The values are the ones the controls defaulted to, so a library that was
 * never told otherwise behaves exactly as it did.
 */

/**
 * How far the skip controls move, and how far an arrow key moves where there is
 * no line to land on.
 *
 * Ten seconds is the interval every player has settled on, which matters more
 * than the number itself: it is what a hand already expects.
 */
export const SKIP_MS = 10_000;

/**
 * The run-up when carrying a film on.
 *
 * Coming back in the middle of a line is disorienting, and a few seconds is
 * enough to remember where you were without watching anything twice.
 */
export const REWIND_MS = 5_000;

/**
 * How long the controls stay up with the pointer still over a playing film.
 *
 * Long enough to move from one control to another, short enough that a film
 * left alone is a film and not a film with a bar across it.
 */
export const HIDE_AFTER_MS = 2_600;

/**
 * How far through a film has to be before it stops being offered to carry on
 * with.
 *
 * Short of the end, because almost nobody sits through the credits and a film
 * left two minutes from the end should not be offered for ever.
 */
export const WATCHED_FRACTION = 0.97;
