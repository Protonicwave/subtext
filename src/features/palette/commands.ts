import { foldedOf } from './matching';

/**
 * The things the application does that are worth a key and a line in the
 * palette.
 *
 * Every one of these is already a control on a screen somewhere. Nothing here
 * is a new thing to do: it is the same thing, reachable without first finding
 * the screen the button is on. That is also why each carries its key. The
 * palette lists what a key would have done, so somebody who came through it
 * twice learns the key and stops coming through it.
 *
 * One list, read by three things: the palette draws it, the window binds it,
 * and the reference table describes it. A key written down in more than one of
 * those would eventually be a key that did something else in one of them.
 */

export type CommandId = 'palette' | 'folder' | 'view' | 'transcript' | 'rescan' | 'settings';

/** Everything but opening the palette, which is the one command it cannot list. */
export type ActionId = Exclude<CommandId, 'palette'>;

export interface Command {
  readonly id: CommandId;
  /** What it does, as the palette lists it and the key table describes it. */
  readonly name: string;
  /**
   * The letter pressed with Ctrl, folded to lower case as a key event reports
   * it. The palette itself is opened by its own and is not listed inside it.
   */
  readonly key: string;
  /** The same, as it is drawn. */
  readonly keys: readonly string[];
}

export const COMMANDS: readonly Command[] = [
  { id: 'palette', name: 'Find a film', key: 'k', keys: ['Ctrl', 'K'] },
  { id: 'folder', name: 'Choose a folder to watch', key: 'o', keys: ['Ctrl', 'O'] },
  { id: 'view', name: 'Show the library as covers or as a list', key: 'l', keys: ['Ctrl', 'L'] },
  /*
   * T for the text of the film, and with Ctrl because the letters on their own
   * belong to the player. It is one key both ways: the panel is put away by the
   * key that brought it out, which is the only arrangement worth remembering.
   */
  {
    id: 'transcript',
    name: 'Show or hide the dialogue beside the film',
    key: 't',
    keys: ['Ctrl', 'T'],
  },
  /*
   * The key the whole world uses for reading something again, which is what
   * this does. The webview would rather reload the window with it, so the
   * binding says it has taken it, and a reload is not a thing a packaged
   * application offers anyway.
   */
  { id: 'rescan', name: 'Read the watched folders again', key: 'r', keys: ['Ctrl', 'R'] },
  { id: 'settings', name: 'Open settings', key: ',', keys: ['Ctrl', ','] },
];

/** A command the palette lists, which is every one but its own. */
export interface Action extends Command {
  readonly id: ActionId;
}

/** Everything but the key that opened the palette, which is what it lists. */
export const ACTIONS: readonly Action[] = COMMANDS.filter(
  (command): command is Action => command.id !== 'palette',
);

/** The command a key press is asking for, or nothing if it is not asking. */
export function commandFor(event: KeyboardEvent): Command | undefined {
  if (event.defaultPrevented || event.altKey || event.shiftKey) return undefined;
  if (!event.ctrlKey && !event.metaKey) return undefined;

  const pressed = event.key.toLowerCase();
  return COMMANDS.find((command) => command.key === pressed);
}

/**
 * The actions a query matches, by name.
 *
 * Every action while nothing has been typed, since a palette that opened empty
 * would be a palette that had to be guessed at. Folded the same way films are,
 * so the two lists answer a query alike.
 */
export function actionsMatching(query: string): readonly Action[] {
  const wanted = foldedOf(query.trim());
  if (wanted === '') return ACTIONS;

  return ACTIONS.filter((action) => foldedOf(action.name).includes(wanted));
}
