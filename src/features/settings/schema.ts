import type { PreferenceView } from '@/shared/ipc/bindings';
import { PLAYBACK, SUBTITLES } from '@/features/player/defaults';

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

interface Field<T> {
  readonly key: string;
  readonly fallback: T;
  /** What the stored text means, or nothing if it means nothing. */
  readonly read: (stored: string) => T | undefined;
}

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
function amount(key: string, least: number, most: number, fallback: number): Field<number> {
  return {
    key,
    fallback,
    read: (stored) => {
      const value = Number(stored);
      if (stored.trim() === '' || !Number.isFinite(value)) return undefined;
      return Math.min(most, Math.max(least, value));
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
  { value: SUBTITLES.colour, name: 'Paper' },
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

  subtitleTypeface: choice('subtitles.typeface', ['sans', 'serif'], SUBTITLES.typeface),
  subtitleSize: amount('subtitles.size', 2.4, 8, SUBTITLES.size),
  subtitleWeight: amount('subtitles.weight', 400, 700, SUBTITLES.weight),
  subtitleColour: choice(
    'subtitles.colour',
    SUBTITLE_COLOURS.map((colour) => colour.value),
    SUBTITLES.colour,
  ),
  subtitleBackground: choice(
    'subtitles.background',
    ['none', 'shadow', 'panel'],
    SUBTITLES.background,
  ),
  subtitlePosition: amount('subtitles.position', 2, 24, SUBTITLES.position),

  resume: choice('playback.resume', ['carry-on', 'beginning'], 'carry-on'),
  rewindMs: amount('playback.rewind', 0, 30_000, PLAYBACK.rewindMs),
  skipMs: amount('playback.skip', 5_000, 60_000, PLAYBACK.skipMs),
  dialogueArrows: toggle('playback.arrows', PLAYBACK.dialogueArrows),
  watchedFraction: amount('playback.watched', 0.8, 1, PLAYBACK.watchedFraction),
  hideAfterMs: amount('playback.hide', 1_000, 10_000, PLAYBACK.hideAfterMs),
  hardwareDecoding: toggle('playback.hardware', true),

  accent: choice('appearance.accent', ['film', 'fixed'], 'film'),
  glow: amount('appearance.glow', 0, 1.6, 1),
  grain: amount('appearance.grain', 0, 0.09, GRAIN),
  motion: choice('appearance.motion', ['system', 'reduced'], 'system'),
  transcriptTypeface: choice('transcript.typeface', ['serif', 'sans'], 'serif'),
  transcriptFollow: toggle('transcript.follow', true),
} as const;

type Held<F> = F extends Field<infer T> ? T : never;

/** Everything the application has been told to prefer. */
export type Settings = { [Name in keyof typeof FIELDS]: Held<(typeof FIELDS)[Name]> };

export type SettingName = keyof Settings;

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
