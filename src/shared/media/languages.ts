/**
 * The languages a subtitle track can say it is in.
 *
 * The list is the one the pairing reads out of file names, in
 * `crates/subtext-core/src/pairing/language.rs`, and a test beside this file
 * reads that file to make sure the two have not drifted. A code that is offered
 * here but never produced there would be a preference nothing could satisfy.
 *
 * The names are not written down. The platform already knows what a code is
 * called and will say it in the language the person is reading, which is one
 * table fewer to keep and thirty fewer chances to be wrong about somebody
 * else's language.
 */

/** Every two letter code the pairing recognises, in the order it lists them. */
export const LANGUAGE_CODES = [
  'ar',
  'bg',
  'cs',
  'da',
  'de',
  'el',
  'en',
  'es',
  'fa',
  'fi',
  'fr',
  'he',
  'hi',
  'hr',
  'hu',
  'id',
  'it',
  'ja',
  'ko',
  'nl',
  'no',
  'pl',
  'pt',
  'ro',
  'ru',
  'sr',
  'sv',
  'th',
  'tr',
  'uk',
  'vi',
  'zh',
] as const;

/** Any of them, which is what a preference for none in particular says. */
export const ANY_LANGUAGE = 'any';

/**
 * Built once. It reads a data table on the way up, and the track menu asks it
 * for a name every time it is drawn.
 */
const NAMES = displayNames();

function displayNames(): Intl.DisplayNames | null {
  try {
    return new Intl.DisplayNames(undefined, { type: 'language', fallback: 'none' });
  } catch {
    // A platform without the data, which is a webview built without it rather
    // than anything a person did. The codes are still readable.
    return null;
  }
}

/**
 * What a language code is called, or the code itself where nothing knows.
 *
 * Takes whatever the track carries rather than only the codes above, since a
 * file can be labelled in ways the pairing did not recognise and naming it
 * badly is better than showing a track with no name at all.
 */
export function languageNamed(code: string): string {
  const trimmed = code.trim();
  if (trimmed === '') return '';

  const named = NAMES?.of(trimmed);
  if (named === undefined || named === '') return trimmed.toUpperCase();

  // Capitalised, because the platform gives the name as it is written mid
  // sentence and this one begins a row.
  return named.charAt(0).toUpperCase() + named.slice(1);
}
