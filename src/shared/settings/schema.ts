import type { PreferenceView } from '@/shared/ipc/bindings';

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
  // The value Rust reads under this key is the name of the strict one, so these
  // two spellings are shared with `src-tauri/src/settings.rs`.
  matching: choice('library.matching', ['relaxed', 'exact'], 'relaxed'),

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

  resume: choice('playback.resume', ['carry-on', 'beginning'], 'carry-on'),
  /*
   * Coming back to a film in the middle of a line is disorienting, and a few
   * seconds of run-up is enough to remember where you were.
   */
  rewindMs: amount('playback.rewind', { least: 0, most: 30_000, step: 1_000 }, 5_000),
  skipMs: amount('playback.skip', { least: 5_000, most: 60_000, step: 5_000 }, 10_000),
  /*
   * On, because it is the better answer to what somebody pressing the left
   * arrow wants: they missed a line, and the line is where they meant to go.
   */
  dialogueArrows: toggle('playback.arrows', true),
  /*
   * Not all of it, because almost nobody sits through the credits and a film
   * left two minutes from the end should not be offered for ever.
   */
  watchedFraction: amount('playback.watched', { least: 0.8, most: 1, step: 0.01 }, 0.97),
  hideAfterMs: amount('playback.hide', { least: 1_000, most: 10_000, step: 200 }, 2_600),
  hardwareDecoding: toggle('playback.hardware', true),

  accent: choice('appearance.accent', ['film', 'fixed'], 'film'),
  glow: amount('appearance.glow', { least: 0, most: 1.6, step: 0.05 }, 1),
  grain: amount('appearance.grain', { least: 0, most: 0.09, step: 0.005 }, GRAIN),
  motion: choice('appearance.motion', ['system', 'reduced'], 'system'),
  transcriptTypeface: choice('transcript.typeface', ['serif', 'sans'], 'serif'),
  transcriptFollow: toggle('transcript.follow', true),
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
