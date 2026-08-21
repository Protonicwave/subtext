import { Choice } from './controls';

/**
 * How the library is drawn, which is a different question from where it comes
 * from.
 *
 * The four here are one decision seen from four sides: how the films are
 * grouped, whether they are covers or a table, how large a cover is, and what a
 * film with no artwork is drawn as. The first three are also reachable from the
 * library screen itself, because that is where somebody is standing when they
 * want them changed. They are here as well so that the screen has somewhere to
 * say what each one means.
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
        label="How the films are drawn"
        hint="Covers are what a library is for. A list serves one too large to look at, with a column for each thing you might sort by. Spines are the whole shelf at once, each film an edge in its own colours, for finding one by sight."
        options={[
          { value: 'covers', label: 'Covers' },
          { value: 'list', label: 'List' },
          { value: 'spines', label: 'Spines' },
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

      <Choice
        name="withoutArtwork"
        label="Films with no artwork"
        hint="Subtext looks for a picture inside each film and beside it on the disk, and never anywhere else. A film that has none is drawn from its own title, in its own colours. A frame from the film is the other answer: it shows what the film looks like, taken a fifth of the way in by nobody in particular."
        options={[
          { value: 'composed', label: 'A composed cover' },
          { value: 'frame', label: 'A frame from the film' },
        ]}
      />
    </>
  );
}
