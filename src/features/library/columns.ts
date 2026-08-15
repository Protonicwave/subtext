import type { FilmView } from '@/shared/ipc/bindings';
import type { Settings } from '@/shared/settings/schema';

/**
 * The columns of the list, and the order each one puts the library in.
 *
 * A column sorts by something other than what it shows almost every time: the
 * size column shows 15.8 GB and orders by bytes, the runtime column shows two
 * hours and orders by milliseconds, and the watched column shows a bar and
 * orders by how much of it is filled. So a column here is what it is called,
 * whether it is figures, and what it sorts by. What it draws belongs with the
 * drawing.
 *
 * Which column is in force is a setting, so the names come from the settings
 * table rather than from here: a column with nowhere to be remembered would be
 * a column that forgot itself on the next run.
 */

export type SortName = Settings['librarySort'];

/** What a column orders by, or nothing where the film does not say. */
type Key = string | number | null;

interface Column {
  /** The heading, which is also what the control to sort by it announces. */
  name: string;
  /** Figures, which are set in the mono face and aligned right. */
  figures: boolean;
  of: (film: FilmView) => Key;
}

export const COLUMNS: Record<SortName, Column> = {
  title: { name: 'Title', figures: false, of: (film) => sortableTitle(film.title) },
  folder: { name: 'Folder', figures: false, of: (film) => film.shelf.name },
  year: { name: 'Year', figures: true, of: (film) => film.year },
  runtime: { name: 'Runtime', figures: true, of: (film) => film.durationMs },
  /*
   * Ordered by how tall the picture is rather than by what the codec is called,
   * which is the only orderable fact in the column. Sorting by name would put
   * AV1 above H.264 for no reason anybody asked for, whereas somebody sorting
   * this column is looking for what they have in ultra high definition.
   */
  video: { name: 'Video', figures: false, of: (film) => film.details?.video?.height ?? null },
  size: { name: 'Size', figures: true, of: (film) => film.details?.sizeBytes ?? null },
  added: { name: 'Added', figures: true, of: (film) => film.addedAt },
  watched: { name: 'Watched', figures: false, of: watchedOf },
};

/** The columns as they are drawn, left to right. */
export const ORDER: readonly SortName[] = [
  'title',
  'folder',
  'year',
  'runtime',
  'video',
  'size',
  'added',
  'watched',
];

/**
 * How far through a film somebody is, from zero to one.
 *
 * A film that was finished is all of it, whatever second it was stopped at:
 * finishing is what the position says happened, and the seconds left over are
 * the credits.
 */
function watchedOf(film: FilmView): number | null {
  if (film.position === null) return null;
  return film.position.finished ? 1 : film.position.progress;
}

/**
 * The library in the order a column puts it.
 *
 * The keys are worked out once each rather than once per comparison, because
 * ten thousand films are a hundred and thirty thousand comparisons and reading
 * a title out of a film that many times is most of the work. What is sorted is
 * the keys, and the films come back out at the end.
 */
export function sortedBy(
  films: readonly FilmView[],
  name: SortName,
  descending: boolean,
): FilmView[] {
  const column = COLUMNS[name];
  const keyed: Keyed[] = films.map((film) => {
    const key = column.of(film);
    return {
      film,
      key: typeof key === 'string' ? folded(key) : key,
      // Every sort settles ties by title, so that two films of the same year
      // are in an order somebody can predict rather than in the order they came
      // out of the database.
      filed: folded(sortableTitle(film.title)),
    };
  });

  const direction = descending ? -1 : 1;
  keyed.sort((one, other) => {
    // A film with no year is not a film in year zero, so it goes last whichever
    // way round the column is. Reversing that would bury every film that is
    // known under every film that is not, and the answer somebody wanted from
    // sorting by newest first would be off the bottom of the window.
    if (one.key === null || other.key === null) {
      if (one.key === other.key) return settle(one, other);
      return one.key === null ? 1 : -1;
    }

    const between = compare(one.key, other.key) * direction;
    return between === 0 ? settle(one, other) : between;
  });

  return keyed.map((held) => held.film);
}

/**
 * A title as it is filed, which is without the article in front of it.
 *
 * The Conversation belongs under C, the way it does on a shelf and in every
 * catalogue anybody has used. The full title still settles ties, so two films
 * that differ only by an article keep a stable order rather than swapping about
 * between runs.
 */
export function sortableTitle(title: string): string {
  const filed = title.replace(ARTICLE, '').trim();
  return filed === '' ? title : filed;
}

const ARTICLE = /^(?:the|a|an)\s+/i;

interface Keyed {
  film: FilmView;
  key: string | number | null;
  filed: string;
}

/**
 * A name as it is compared, which is not as it is written.
 *
 * Folded once for each film rather than compared through a collator on every
 * step. A collator over ten thousand titles is a hundred and thirty thousand
 * calls into the platform and most of the time a sort takes; folding is ten
 * thousand passes over a short string, after which the comparison is the one
 * between two primitives.
 *
 * Three things happen here. Accents come off, so a library is in one order
 * rather than in two. Case goes, since nobody thinks of ALIEN as being filed
 * apart from Alien. And a run of digits is padded, so that Alien 2 comes before
 * Alien 10 rather than after it. A run longer than the padding is left as it is,
 * which is a number no film has in its title.
 */
function folded(name: string): string {
  return name
    .normalize('NFD')
    .replace(ACCENTS, '')
    .toLowerCase()
    .replace(DIGITS, (run) => run.padStart(DIGIT_WIDTH, '0'));
}

const ACCENTS = /[\u0300-\u036f]/g;
const DIGITS = /\d+/g;
const DIGIT_WIDTH = 12;

/**
 * Which of two films comes first when the column cannot tell them apart.
 *
 * The filed title first and the whole title after it, so that two films that
 * differ only by an article are in the same order however the library was
 * handed over. Falling back on the order they arrived in would put them one way
 * round after a scan and the other way round after the next one.
 */
function settle(one: Keyed, other: Keyed): number {
  if (one.filed !== other.filed) return one.filed < other.filed ? -1 : 1;
  if (one.film.title === other.film.title) return 0;
  return one.film.title < other.film.title ? -1 : 1;
}

function compare(one: string | number, other: string | number): number {
  if (typeof one === 'number' && typeof other === 'number') return one - other;
  if (one === other) return 0;
  return one < other ? -1 : 1;
}
