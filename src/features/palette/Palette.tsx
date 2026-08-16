import {
  Fragment,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as KeyPress,
  type RefObject,
  type SyntheticEvent,
} from 'react';
import type { FilmView } from '@/shared/ipc/bindings';
import type { Settings } from '@/shared/settings/schema';
import { useSetting } from '@/shared/settings/useSettings';
import { SearchIcon } from '@/shared/ui/Icon';
import { paletteFor } from '@/features/library/accent';
import { useLibrary } from '@/features/library/useLibrary';
import { useSheet } from '@/features/library/useSheet';
import { actionsMatching, commandFor, type Action } from './commands';
import { entriesOf, filmsMatching, movedBy } from './matching';
import { runCommand } from './running';
import styles from './Palette.module.css';

/**
 * Any film, or anything the application does, one keystroke away.
 *
 * Mounted for the life of the window rather than put up when it is wanted,
 * because the key that opens it has to work from every screen and a component
 * that is not there cannot listen for one.
 *
 * The field and the list are a combobox rather than a text box with buttons
 * under it. Focus never leaves the field, which is what lets somebody keep
 * typing while the arrows move through what they have found, and the row being
 * considered is named to a screen reader rather than only drawn differently.
 */

/** One line of the list, whichever of the two kinds it is. */
type Row =
  | { readonly kind: 'film'; readonly film: FilmView }
  | { readonly kind: 'action'; readonly action: Action };

export function Palette() {
  const dialog = useRef<HTMLDialogElement>(null);
  const field = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);

  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);

  const films = useLibrary((library) => library.films);
  const accent = useSetting('accent');
  const openSheet = useSheet((sheet) => sheet.open);

  /*
   * Folded once, when the library changes rather than when a key is pressed.
   * The library is read whole on every refresh, so this is a new array on each
   * scan and the same one between them, which is exactly when it should be
   * rebuilt.
   */
  const entries = useMemo(() => entriesOf(films), [films]);

  const found = useMemo(() => filmsMatching(entries, query), [entries, query]);
  const actions = useMemo(() => actionsMatching(query), [query]);

  // Films first, then what the application can do, as one list: the arrows walk
  // from the last film onto the first action without stopping at the heading
  // between them, and the selection stays one number rather than a pair.
  const rows = useMemo<Row[]>(
    () => [
      ...found.films.map((film): Row => ({ kind: 'film', film })),
      ...actions.map((action): Row => ({ kind: 'action', action })),
    ],
    [found, actions],
  );

  // The first result, always, since the list under a new query is a different
  // list and an index into the last one points at nothing in particular.
  const at = selected < rows.length ? selected : 0;

  const show = useCallback(() => {
    setOpen(true);
  }, []);

  // Emptied on the way out rather than on the way in, so that the palette is
  // always opened blank and nothing has to be cleared before it is drawn.
  const hide = useCallback(() => {
    setOpen(false);
    setQuery('');
    setSelected(0);
  }, []);

  useCommandKeys(open, show, hide);
  useShown(dialog, field, open);
  useFollowsSelection(list, at);

  const choose = (row: Row | undefined) => {
    if (row === undefined) return;
    hide();

    // Closed first either way. The film's page comes forward over whatever was
    // behind it, and an action that opens the folder picker or the settings
    // screen has nothing to say to a palette still on top of it.
    if (row.kind === 'film') openSheet(row.film.id);
    else runCommand(row.action.id);
  };

  const onKeyDown = (event: KeyPress<HTMLDialogElement>) => {
    switch (event.key) {
      case 'Escape':
        // Taken here so that the shell does not read it as well and go back a
        // screen behind the palette that is closing.
        hide();
        break;
      case 'ArrowDown':
        setSelected(movedBy(at, 1, rows.length));
        break;
      case 'ArrowUp':
        setSelected(movedBy(at, -1, rows.length));
        break;
      case 'Home':
        setSelected(0);
        break;
      case 'End':
        setSelected(Math.max(0, rows.length - 1));
        break;
      case 'Enter':
        choose(rows[at]);
        break;
      default:
        return;
    }

    event.preventDefault();
  };

  const typed = query.trim() !== '';
  const firstAction = found.films.length;

  return (
    <dialog
      ref={dialog}
      className={styles.backdrop}
      aria-label="Find a film"
      onKeyDown={onKeyDown}
      onClick={(event: SyntheticEvent<HTMLDialogElement>) => {
        if (event.target === dialog.current) hide();
      }}
      onCancel={(event) => {
        event.preventDefault();
      }}
    >
      <div className={styles.palette}>
        <div className={styles.field}>
          <SearchIcon size={14} />
          <input
            ref={field}
            type="text"
            className={styles.input}
            value={query}
            placeholder="Find a film, or do something"
            aria-label="Find a film"
            role="combobox"
            aria-expanded={rows.length > 0}
            aria-controls={LIST}
            aria-activedescendant={rows.length > 0 ? optionId(at) : undefined}
            aria-autocomplete="list"
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => {
              setQuery(event.target.value);
              setSelected(0);
            }}
          />
        </div>

        <div ref={list} className={styles.body}>
          <div className={styles.list} role="listbox" id={LIST} aria-label="Films and actions">
            {rows.map((row, index) => (
              <Fragment key={row.kind === 'film' ? `film-${String(row.film.id)}` : row.action.id}>
                {/* Named to the list rather than to a screen reader, which is
                    told what a row is by the row itself. */}
                {index === 0 && row.kind === 'film' && (
                  <p role="presentation" className={styles.group}>
                    Films
                  </p>
                )}
                {index === firstAction && (
                  <p role="presentation" className={styles.group}>
                    {typed ? 'Do something' : 'What Subtext can do'}
                  </p>
                )}

                <div
                  id={optionId(index)}
                  role="option"
                  aria-selected={index === at}
                  className={styles.row}
                  /* The pointer moving over a row is somebody considering it,
                     so Enter should take the one under the cursor rather than
                     the one the arrows left behind. */
                  onPointerMove={() => {
                    setSelected(index);
                  }}
                  onClick={() => {
                    choose(row);
                  }}
                >
                  {row.kind === 'film' ? (
                    <FilmRow film={row.film} accent={accent} />
                  ) : (
                    <>
                      <span className={styles.what}>{row.action.name}</span>
                      <kbd className={styles.keys}>{row.action.keys.join(' ')}</kbd>
                    </>
                  )}
                </div>

                {index === firstAction - 1 && found.withheld > 0 && (
                  <p role="presentation" className={styles.withheld}>
                    and {found.withheld.toLocaleString('en-GB')} more. Type a little more of the
                    title to narrow it.
                  </p>
                )}
              </Fragment>
            ))}
          </div>

          {typed && found.films.length === 0 && (
            <p className={styles.nothing}>
              Nothing in your films is called that. {countOf(films.length)}
            </p>
          )}
        </div>

        <footer className={styles.hints}>
          <span>
            <kbd className={styles.keys}>↑↓</kbd> to move
          </span>
          <span>
            <kbd className={styles.keys}>Enter</kbd> to open
          </span>
          <span>
            <kbd className={styles.keys}>Esc</kbd> to close
          </span>
        </footer>
      </div>
    </dialog>
  );
}

/** One film: what it is called, when it is from, and where it is filed. */
function FilmRow({ film, accent }: { film: FilmView; accent: Settings['accent'] }) {
  const palette = paletteFor(film, accent);

  return (
    <>
      <span
        className={styles.chip}
        aria-hidden="true"
        style={{ '--film-accent': palette.primary, '--film-pair': palette.pair } as CSSProperties}
      />
      <span className={styles.what}>{film.title}</span>
      {film.year !== null && <span className={styles.year}>{film.year}</span>}
      <span className={styles.where}>{film.shelf.name}</span>
    </>
  );
}

/** The list, which the field points at. One palette, so one identifier. */
const LIST = 'palette-list';

function optionId(at: number): string {
  return `palette-row-${String(at)}`;
}

/** What there was to look through, for a query that found none of it. */
function countOf(films: number): string {
  if (films === 0) return 'There are no films in the library yet.';
  if (films === 1) return 'There is one film in the library.';
  return `There are ${films.toLocaleString('en-GB')} films in the library.`;
}

/**
 * Ctrl+K from anywhere, and the other commands with it.
 *
 * Bound to the window rather than to anything on screen, which is the point of
 * them. The palette listens for all of them because it is the one component
 * mounted for the life of the window that is about them, and because a key that
 * worked while the palette was down but not while it was up would be a key that
 * behaved differently depending on what could be seen.
 */
function useCommandKeys(open: boolean, show: () => void, hide: () => void) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const command = commandFor(event);
      if (command === undefined) return;

      if (command.id === 'palette') {
        if (open) hide();
        else show();
      } else {
        runCommand(command.id);
        if (open) hide();
      }

      event.preventDefault();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [open, show, hide]);
}

/**
 * Puts the palette on the top layer and takes it off again.
 *
 * The platform's own modal dialog rather than an overlay of ours: it traps
 * focus, it takes everything behind it out of the tab order and away from a
 * screen reader, and no stacking context on the page can get above it. Focus
 * goes back where it was found on the way out, so closing leaves somebody on
 * the tile or the control they opened it from.
 */
function useShown(
  dialog: RefObject<HTMLDialogElement | null>,
  field: RefObject<HTMLInputElement | null>,
  open: boolean,
) {
  const opener = useRef<Element | null>(null);

  useEffect(() => {
    const element = dialog.current;
    if (element === null) return;

    if (open && !element.open) {
      opener.current = document.activeElement;
      element.showModal();
      // Named rather than left to the dialog's own guess at what to focus,
      // since every key the palette reads is read from the field.
      field.current?.focus();
    }

    if (!open && element.open) {
      element.close();
      const came = opener.current;
      if (came instanceof HTMLElement && came.isConnected) came.focus();
    }
  }, [dialog, field, open]);
}

/** Keeps the selected row in view as the arrows walk past the edge of it. */
function useFollowsSelection(list: RefObject<HTMLDivElement | null>, selected: number) {
  useEffect(() => {
    list.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: 'nearest', behavior: 'auto' });
  }, [list, selected]);
}
