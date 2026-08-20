import type { PreferenceView } from '@/shared/ipc/bindings';
import type { Comfort } from '@/shared/media/cues';
import { ANY_LANGUAGE, LANGUAGE_CODES } from '@/shared/media/languages';

/**
 * Every preference: where it is kept, what it may be, and what the application
 * does without one.
 *
 * One table rather than a type here, a default there and a parser somewhere
 * else. Adding a control is adding a line to it, and nothing can then be
 * written without a default or read without being checked.
 *
 * What is stored is text, because that is what the preference table holds and
 * what the two keys Rust reads for itself expect to find. Anything the table
 * does not recognise reads as the default: a value can only have got there by
 * being edited by hand or written by a later version, and neither is worth
 * refusing to open the window over.
 */

/**
 * How the subtitles are drawn, as the renderer takes them.
 *
 * The renderer decides nothing for itself. Everything about how a line looks
 * arrives as one of these, which is what makes the settings screen a matter of
 * where the values come from rather than of rewriting what reads them.
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

interface Field<T> {
  readonly key: string;
  readonly fallback: T;
  /** What the stored text means, or nothing if it means nothing. */
  readonly read: (stored: string) => T | undefined;
}

/** How far a number may go, and how finely it moves between the two. */
export interface Range {
  readonly least: number;
  readonly most: number;
  readonly step: number;
}

/** A number, and the range the control for it is drawn across. */
interface Amount extends Field<number>, Range {}

/** One of a fixed set of names. */
function choice<const T extends string>(key: string, options: readonly T[], fallback: T): Field<T> {
  return {
    key,
    fallback,
    read: (stored) => options.find((option) => option === stored),
  };
}

function toggle(key: string, fallback: boolean): Field<boolean> {
  return {
    key,
    fallback,
    read: (stored) => (stored === 'true' ? true : stored === 'false' ? false : undefined),
  };
}

/**
 * A number between two bounds.
 *
 * Read back through the same bounds it was written within, so that a value from
 * a version whose slider went further does not put the subtitles off the bottom
 * of the picture.
 */
function amount(key: string, range: Range, fallback: number): Amount {
  return {
    key,
    fallback,
    ...range,
    read: (stored) => {
      const value = Number(stored);
      if (stored.trim() === '' || !Number.isFinite(value)) return undefined;
      return Math.min(range.most, Math.max(range.least, value));
    },
  };
}

/**
 * What a subtitle may be drawn in.
 *
 * A fixed set rather than a colour picker. These end up in a custom property
 * over the picture, the useful range is narrow, and five that were chosen
 * against a film are worth more than sixteen million that were not.
 */
export const SUBTITLE_COLOURS = [
  { value: '#f2f3f6', name: 'Paper' },
  { value: '#ffffff', name: 'White' },
  { value: '#efe3c9', name: 'Warm' },
  { value: '#ffd98a', name: 'Amber' },
  { value: '#b7c0cf', name: 'Cool' },
] as const;

/** What the grain is set to before anybody has said otherwise. Matches the token. */
const GRAIN = 0.035;

export const FIELDS = {
  missingFilms: choice('library.missing', ['keep', 'hide'], 'keep'),
  /*
   * Shelved, because the folders somebody made say something about their films
   * and one wall of two thousand posters says nothing. The wall is still there
   * for a library that was never sorted, or for anybody who would rather see
   * all of it at once.
   */
  libraryLayout: choice('library.layout', ['shelves', 'wall'], 'shelves'),
  /*
   * Covers, a list, or spines. The wall stops serving somewhere around two
   * hundred films, a table serves ten thousand of them by name, and the spines
   * are the other half of that: the whole shelf at once, as edges, for anybody
   * who knows their library by sight rather than by title. Three answers rather
   * than one being right for every library.
   */
  libraryView: choice('library.view', ['covers', 'list', 'spines'], 'covers'),
  /*
   * Which column the list is ordered by, and whether it runs the other way.
   * The names are the columns themselves, so that a sort a version from later
   * knows about and this one does not reads as the title rather than as
   * nothing.
   */
  librarySort: choice(
    'library.sort',
    ['title', 'folder', 'year', 'runtime', 'video', 'size', 'added', 'watched'],
    'title',
  ),
  libraryDescending: toggle('library.descending', false),
  /*
   * How large a cover is drawn, on the wall and on the rails alike. Three named
   * sizes rather than a slider: what is being chosen is how many films fit
   * across the window against how well each one can be read, and three answers
   * cover that where a hundred would only make it harder to choose.
   */
  tileSize: choice('library.tiles', ['small', 'medium', 'large'], 'medium'),
  /*
   * What a film with no artwork anywhere on the disk is drawn as.
   *
   * The composed cover, because it was designed and the frame is a guess: a
   * fifth of the way into a film is a moment nobody chose, and a wall of those
   * reads as a decoder having run rather than as a shelf of films. The frame is
   * still taken, and still shown where a wide picture is wanted, so this
   * decides what a cover is and nothing else.
   */
  withoutArtwork: choice('library.without-artwork', ['composed', 'frame'], 'composed'),
  // The value Rust reads under this key is the name of the strict one, so these
  // two spellings are shared with `src-tauri/src/settings.rs`.
  matching: choice('library.matching', ['relaxed', 'exact'], 'relaxed'),

  /*
   * Which track a film opens on, where nobody has chosen one for it and the
   * pairing found more than one. It decides nothing else: a film that has been
   * chosen for keeps what it was given whatever this says.
   */
  subtitleLanguage: choice('subtitles.language', [ANY_LANGUAGE, ...LANGUAGE_CODES], ANY_LANGUAGE),
  subtitleTypeface: choice('subtitles.typeface', ['sans', 'serif'], 'sans'),
  subtitleSize: amount('subtitles.size', { least: 2.4, most: 8, step: 0.1 }, 4.4),
  subtitleWeight: amount('subtitles.weight', { least: 400, most: 700, step: 100 }, 500),
  subtitleColour: choice(
    'subtitles.colour',
    SUBTITLE_COLOURS.map((colour) => colour.value),
    SUBTITLE_COLOURS[0].value,
  ),
  subtitleBackground: choice('subtitles.background', ['none', 'shadow', 'panel'], 'shadow'),
  subtitlePosition: amount('subtitles.position', { least: 2, most: 24, step: 0.5 }, 7),
  /*
   * A little before the words, which is what broadcast subtitling does and what
   * a line landing on its own first syllable fails to do. Zero for anybody who
   * would rather have the file exactly as it was written.
   */
  subtitleLeadInMs: amount('subtitles.lead', { least: 0, most: 300, step: 10 }, 90),
  /*
   * Five sixths of a second, near enough: long enough that a one word answer
   * can be read, short enough that it rarely reaches the line after it.
   */
  subtitleMinimumMs: amount('subtitles.minimum', { least: 0, most: 2_000, step: 50 }, 850),

  /*
   * Whether the dialogue is listed beside the picture. Hidden until it is
   * asked for, because a film opens as a film, and remembered from then on:
   * somebody who watches with the lines beside them wants them beside them for
   * the next film as well, and the key that shows them is the same key that
   * puts them away.
   */
  transcript: choice('player.transcript', ['shown', 'hidden'], 'hidden'),

  resume: choice('playback.resume', ['carry-on', 'beginning'], 'carry-on'),
  hardwareDecoding: toggle('playback.hardware', true),

  accent: choice('appearance.accent', ['film', 'fixed'], 'film'),
  glow: amount('appearance.glow', { least: 0, most: 1.6, step: 0.05 }, 1),
  grain: amount('appearance.grain', { least: 0, most: 0.09, step: 0.005 }, GRAIN),
  motion: choice('appearance.motion', ['system', 'reduced'], 'system'),
} as const;

type Held<F> = F extends Field<infer T> ? T : never;

/** Everything the application has been told to prefer. */
export type Settings = { [Name in keyof typeof FIELDS]: Held<(typeof FIELDS)[Name]> };

export type SettingName = keyof Settings;

/** The settings holding one kind of value, which one kind of control edits. */
type NamesHolding<T> = {
  [Name in SettingName]: Settings[Name] extends T ? Name : never;
}[SettingName];

export type ToggleName = NamesHolding<boolean>;
export type AmountName = NamesHolding<number>;

/** How far the control for one number may move it. */
export function rangeOf(name: AmountName): Range {
  return FIELDS[name];
}

export const DEFAULTS: Settings = Object.fromEntries(
  Object.entries(FIELDS).map(([name, field]) => [name, field.fallback]),
) as Settings;

/**
 * The settings a library file holds, with the defaults standing in for
 * everything it does not.
 */
export function settingsFrom(stored: readonly PreferenceView[]): Settings {
  const byKey = new Map(stored.map((preference) => [preference.key, preference.value]));

  return Object.fromEntries(
    Object.entries(FIELDS).map(([name, field]) => {
      const text = byKey.get(field.key);
      const value = text === undefined ? undefined : field.read(text);
      return [name, value ?? field.fallback];
    }),
  ) as Settings;
}

/**
 * The keys in a library file that nothing here reads any more.
 *
 * A build that offered a setting this one does not leaves its row behind, and a
 * row nothing reads is a row that accumulates: the preferences a version from
 * two releases ago wrote would still be there in ten years' time.
 *
 * They are dropped rather than kept, which does mean that opening a library
 * with an older build after a newer one loses whatever the newer one had been
 * told. That is the price of not carrying dead keys forward, and it is the
 * cheaper of the two: a setting is one control away from being set again.
 */
export function deadKeysIn(stored: readonly PreferenceView[]): string[] {
  const known = new Set(Object.values(FIELDS).map((field) => field.key));

  return stored.map((preference) => preference.key).filter((key) => !known.has(key));
}

/** One setting as the preference table takes it. */
export function storedAs<Name extends SettingName>(
  name: Name,
  value: Settings[Name],
): { key: string; value: string } {
  // Every setting is a name, a number or a yes, and the table holds text, so
  // there is nothing to write but the value said plainly. Reading it back is
  // the half that needs to know what it is looking at.
  return { key: FIELDS[name].key, value: String(value) };
}

/** How long a line is to be given, as the timeline takes it. */
export function comfortOf(settings: Settings): Comfort {
  return { leadInMs: settings.subtitleLeadInMs, minimumMs: settings.subtitleMinimumMs };
}

/** How the subtitles are to be drawn, as the renderer takes them. */
export function appearanceOf(settings: Settings): SubtitleAppearance {
  return {
    typeface: settings.subtitleTypeface,
    size: settings.subtitleSize,
    weight: settings.subtitleWeight,
    colour: settings.subtitleColour,
    background: settings.subtitleBackground,
    position: settings.subtitlePosition,
  };
}
