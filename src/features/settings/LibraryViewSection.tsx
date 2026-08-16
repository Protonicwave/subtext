import { Choice } from './controls';

/**
 * How the library is drawn, which is a different question from where it comes
 * from.
 *
 * The three here are one decision seen from three sides: how the films are
 * grouped, whether they are covers or a table, and how large a cover is. All
 * three are also reachable from the library screen itself, because that is
 * where somebody is standing when they want them changed. They are here as
 * well so that the screen has somewhere to say what each one means.
 */
export function LibraryViewSection() {
  return (
    <>
      <Choice
        name="libraryLayout"
        label="How the library is arranged"
        hint="Shelves are the folders you already made: one row for each folder inside a watched folder, with what you have not finished at the top. One wall is every film at once."
        options={[
          { value: 'shelves', label: 'Shelves' },
          { value: 'wall', label: 'One wall' },
        ]}
      />

      <Choice
        name="libraryView"
        label="Covers or a list"
        hint="Covers are what a library is for. A list serves one too large to look at, with a column for each thing you might sort by."
        options={[
          { value: 'covers', label: 'Covers' },
          { value: 'list', label: 'List' },
        ]}
      />

      <Choice
        name="tileSize"
        label="Cover size"
        hint="How large each cover is drawn, on the shelves and on the wall alike. Larger reads better across a room; smaller puts more of the library in the window at once."
        options={[
          { value: 'small', label: 'Small' },
          { value: 'medium', label: 'Medium' },
          { value: 'large', label: 'Large' },
        ]}
      />
    </>
  );
}
