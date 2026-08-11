/**
 * How the player behaves before anybody has said otherwise.
 *
 * Every number the player would otherwise have hard coded lives here, and every
 * one of them is something the settings screen will offer later. Keeping them
 * in one file now means the renderer and the controls already take them as
 * values rather than assuming them, so making them adjustable is a matter of
 * where they come from and not of rewriting what reads them.
 */

export interface SubtitleAppearance {
  typeface: 'sans' | 'serif';
  /**
   * As a percentage of the height of the picture, so that the subtitles are the
   * same size against the film whatever the window is doing.
   */
  size: number;
  weight: number;
  colour: string;
  /**
   * What sits behind the text. A shadow reads on almost everything; a panel is
   * for a film with a bright, busy frame; none is for somebody who would rather
   * see the picture.
   */
  background: 'none' | 'shadow' | 'panel';
  /** How far the text sits above the bottom of the picture, as a percentage. */
  position: number;
}

export const SUBTITLES: SubtitleAppearance = {
  typeface: 'sans',
  size: 4.4,
  weight: 500,
  colour: '#f2f3f6',
  background: 'shadow',
  position: 7,
};

export interface PlaybackPreferences {
  /**
   * How far back a resumed film starts from what was saved.
   *
   * Coming back to a film in the middle of a line is disorienting, and a few
   * seconds of run-up is enough to remember where you were.
   */
  rewindMs: number;
  /** What the skip controls, and the arrow keys otherwise, move by. */
  skipMs: number;
  /**
   * Whether the arrow keys land on dialogue rather than on a fixed number of
   * seconds.
   *
   * On, because it is the better answer to what somebody pressing the left
   * arrow wants: they missed a line, and the line is where they meant to go.
   * A film with no subtitles has no lines to land on and gets the seconds.
   */
  dialogueArrows: boolean;
  /**
   * How much of a film counts as having watched it.
   *
   * Not one, because almost nobody sits through the credits, and a film left
   * two minutes from the end should not be offered as something to carry on
   * with for ever.
   */
  watchedFraction: number;
  /** How long the controls stay after the pointer stops moving. */
  hideAfterMs: number;
  /** How often a position is written down while a film is playing. */
  saveEveryMs: number;
}

export const PLAYBACK: PlaybackPreferences = {
  rewindMs: 5_000,
  skipMs: 10_000,
  dialogueArrows: true,
  watchedFraction: 0.97,
  hideAfterMs: 2_600,
  saveEveryMs: 5_000,
};
