import { describe, expect, it } from 'vitest';
import type { CoverSourceView } from '@/shared/ipc/bindings';
import { coverNameOf, coverStatementOf, isChosen } from './covers';

/** Every value the back end can send, so a new one cannot arrive unnamed. */
const SOURCES: CoverSourceView[] = [
  'chosen',
  'in-file',
  'beside',
  'sidecar',
  'folder-above',
  'none',
];

describe('what a cover is said to be', () => {
  it('names every source a film can have', () => {
    for (const source of SOURCES) {
      expect(coverNameOf(source)).not.toBe('');
      expect(coverStatementOf(source)).not.toBe('');
    }
  });

  it('says something different about each of them', () => {
    const names = new Set(SOURCES.map(coverNameOf));
    const statements = new Set(SOURCES.map(coverStatementOf));

    expect(names.size).toBe(SOURCES.length);
    expect(statements.size).toBe(SOURCES.length);
  });

  it('says a film with nothing found is drawn from itself', () => {
    // The one source that is not a picture on the disk, and the one place the
    // wording must not imply that a file was found.
    expect(coverStatementOf('none')).toContain('from the film itself');
  });

  it('counts only a picked cover as a choice to undo', () => {
    expect(isChosen('chosen')).toBe(true);
    for (const source of SOURCES.filter((source) => source !== 'chosen')) {
      expect(isChosen(source)).toBe(false);
    }
  });
});
