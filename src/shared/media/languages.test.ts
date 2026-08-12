import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { LANGUAGE_CODES, languageNamed } from './languages';

/**
 * Where the pairing keeps the languages it can read out of a file name.
 *
 * From the project root, which is where the test runner is started, since the
 * module's own URL is the one the bundler served it under rather than a file.
 */
const PARSER = resolve(process.cwd(), 'crates/subtext-core/src/pairing/language.rs');

/** The two letter codes that file lists, which is the first of each entry. */
function codesInTheParser(): string[] {
  const source = readFileSync(PARSER, 'utf8');
  const table = source.slice(source.indexOf('const LANGUAGES'), source.indexOf('];'));
  return [...table.matchAll(/\("([a-z]{2})",/g)].map((found) => found[1] ?? '');
}

describe('the languages a subtitle can be in', () => {
  /*
   * The two lists are written in different languages and cannot import each
   * other, so this reads one and compares it to the other. Offering a language
   * the pairing never produces would be a preference no track could satisfy,
   * and leaving one out would hide tracks somebody has on disk.
   */
  it('are the ones the pairing reads out of a file name', () => {
    expect([...LANGUAGE_CODES]).toEqual(codesInTheParser());
  });

  it('are named rather than left as codes', () => {
    expect(languageNamed('en')).toBe('English');
    expect(languageNamed('fr')).toBe('French');
  });

  it('names a code nothing recognises as the code itself', () => {
    expect(languageNamed('qq')).toBe('QQ');
    expect(languageNamed('')).toBe('');
  });
});
