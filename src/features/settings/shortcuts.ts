/**
 * Every key the application answers to, written down once.
 *
 * The same list is the table in the settings screen and the card the question
 * mark key brings up, because two lists would be one list and one lie. It is a
 * description of what is bound elsewhere rather than the binding itself: the
 * player owns its keys, and a table that could change them would be a table
 * that had to be consulted before any of them worked.
 */

import { COMMANDS } from '@/features/palette/commands';

export interface Shortcut {
  /** The keys, in the order they are pressed. Drawn one to a box. */
  readonly keys: readonly string[];
  readonly does: string;
}

export interface ShortcutGroup {
  readonly title: string;
  readonly shortcuts: readonly Shortcut[];
}

export const SHORTCUTS: readonly ShortcutGroup[] = [
  {
    title: 'Anywhere',
    shortcuts: [
      // Taken from the commands themselves rather than written out again, since
      // the palette lists the same keys beside the same names and the two
      // drifting apart would leave one of them lying.
      ...COMMANDS.map((command) => ({ keys: command.keys, does: command.name })),
      { keys: ['?'], does: 'Show this list' },
      { keys: ['Esc'], does: 'Close what is open, or go back a screen' },
    ],
  },
  {
    title: 'Watching a film',
    shortcuts: [
      { keys: ['Space'], does: 'Play or pause. K does the same' },
      { keys: ['←'], does: 'Back a line, or by the skip interval' },
      { keys: ['→'], does: 'On a line, or by the skip interval' },
      { keys: ['M'], does: 'Mute' },
      { keys: ['F'], does: 'Full screen' },
      { keys: ['['], does: 'Subtitles a little earlier' },
      { keys: [']'], does: 'Subtitles a little later' },
      { keys: ['S'], does: 'Show or hide the subtitle timing controls' },
      { keys: ['A'], does: 'Listen to the film and line the subtitles up with it' },
      { keys: ['V'], does: 'Watch a measurement land, and keep it or put it back' },
      { keys: ['C'], does: 'Choose which subtitle to read, or none' },
    ],
  },
];
