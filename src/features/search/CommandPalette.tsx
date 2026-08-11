import {
  Fragment,
  type KeyboardEvent as KeyPress,
  type RefObject,
  type SyntheticEvent,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from 'react';
import { SearchIcon } from '@/shared/ui/Icon';
import { clockOf } from '@/shared/media/clock';
import { useNavigation } from '@/app/routes';
import { linesOf, useLibrary } from '@/features/library/useLibrary';
import { type ResultRow, movedBy, rowsOf } from './rows';
import { foundSaying, nothingSaying } from './saying';
import { SETTLE_MS, useSearch } from './useSearch';
import styles from './CommandPalette.module.css';

/**
 * Any line, any film, one keystroke.
 *
 * Mounted for the life of the application rather than put up when it is
 * wanted, because the shortcut that opens it has to work from anywhere and a
 * component that is not there cannot listen for one.
 *
 * The list is not virtualised, unlike the transcript and the poster grid. A
 * search returns at most a hundred lines by the time it reaches here, and a
 * hundred rows is a list the platform draws without being asked twice.
 *
 * The field and the list are a combobox rather than a text box with some
 * buttons under it. Focus never leaves the field, which is what lets somebody
 * keep typing while the arrows move through what they have found so far, and
 * the line being considered is named to a screen reader rather than merely
 * drawn differently.
 */

/** Opens the palette. With the shift key, over the whole library. */
const OPENS = 'k';

export function CommandPalette() {
  const dialog = useRef<HTMLDialogElement>(null);
  const field = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const listId = useId();

  const open = useSearch((search) => search.open);
  const query = useSearch((search) => search.query);
  const film = useSearch((search) => search.film);
  const scope = useSearch((search) => search.scope);
  const results = useSearch((search) => search.results);
  const answered = useSearch((search) => search.answered);
  const searching = useSearch((search) => search.searching);
  const problem = useSearch((search) => search.problem);
  const recent = useSearch((search) => search.recent);
  const hide = useSearch((search) => search.hide);
  const type = useSearch((search) => search.type);
  const toggleScope = useSearch((search) => search.toggleScope);
  const run = useSearch((search) => search.run);
  const chose = useSearch((search) => search.chose);
  const forget = useSearch((search) => search.forget);

  const openFilm = useNavigation((navigation) => navigation.openFilm);
  const films = useLibrary((library) => library.films);

  useOpensAnywhere();

  const rows = useMemo(() => rowsOf(results), [results]);
  const typed = query.trim() !== '';
  // Before anything is typed the palette offers what was searched for before,
  // and those are what the arrows move through instead.
  const options: readonly unknown[] = typed ? rows : recent;
  const choices = options.length;

  // The selection is kept with the list it was made against, and read back
  // only while that is still the list on screen. A search that has come back
  // since is a different set of lines, on which an index from the last one
  // points at nothing in particular, so it starts again at the top.
  const [selection, setSelection] = useState<{ at: number; of: readonly unknown[] }>({
    at: 0,
    of: options,
  });
  const selected = selection.of === options ? selection.at : 0;
  const select = (at: number) => {
    setSelection({ at, of: options });
  };

  const optionId = (at: number) => `${listId}-${String(at)}`;

  // The search itself, sent once the field has been still long enough to think
  // the word was finished. Leaving cancels it, so a query typed straight past
  // is never sent at all.
  useEffect(() => {
    if (!open || !typed) return;

    const timer = setTimeout(() => void run(), SETTLE_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [open, typed, query, scope, run]);

  useShown(dialog, field, open);
  useFollowsSelection(list, selected);

  const choose = (row: ResultRow | undefined) => {
    if (row === undefined) return;

    // Written down before the palette closes, because closing empties the
    // field this reads the search from.
    void chose();
    openFilm(row.filmId, row.startMs);
    hide();
  };

  const onKeyDown = (event: KeyPress<HTMLDialogElement>) => {
    switch (event.key) {
      case 'Escape':
        // Taken here so that the shell does not read it as well and go back a
        // screen behind the palette that is closing.
        hide();
        break;
      case 'ArrowDown':
        select(movedBy(selected, 1, choices));
        break;
      case 'ArrowUp':
        select(movedBy(selected, -1, choices));
        break;
      case 'Home':
        select(0);
        break;
      case 'End':
        select(Math.max(0, choices - 1));
        break;
      case 'Enter':
        if (typed) choose(rows[selected]);
        else {
          const before = recent[selected];
          if (before !== undefined) type(before);
        }
        break;
      case OPENS:
      case OPENS.toUpperCase():
        // Widens the search to the library and puts it back again. Plain
        // Ctrl+K is what closes the palette, and the window handles that.
        if ((!event.ctrlKey && !event.metaKey) || !event.shiftKey) return;
        toggleScope();
        break;
      default:
        return;
    }

    event.preventDefault();
  };

  const scoped = scope === null ? undefined : films.find((known) => known.id === scope);
  const library = {
    films: films.length,
    lines: films.reduce((total, known) => total + linesOf(known), 0),
  };
  const showRecent = !typed && recent.length > 0;

  return (
    <dialog
      ref={dialog}
      className={styles.backdrop}
      aria-label="Search dialogue"
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
            placeholder={
              scoped === undefined ? 'Search every line in your films' : `Search ${scoped.title}`
            }
            aria-label="Search dialogue"
            role="combobox"
            aria-expanded={choices > 0}
            aria-controls={listId}
            aria-activedescendant={choices > 0 ? optionId(selected) : undefined}
            aria-autocomplete="list"
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => {
              type(event.target.value);
            }}
          />

          {searching && <span className={styles.working} aria-hidden="true" />}

          {film !== null && (
            <button
              type="button"
              className={styles.scope}
              aria-pressed={scope === null}
              onClick={toggleScope}
            >
              {scope === null ? 'Everything' : 'This film'}
              <kbd className={styles.keys}>Ctrl ⇧ K</kbd>
            </button>
          )}
        </div>

        <div ref={list} className={styles.body}>
          {problem !== null && (
            <p role="alert" className={styles.problem}>
              {problem}
            </p>
          )}

          {showRecent && (
            <div className={styles.groupTop}>
              <p className={styles.group}>Searched before</p>
              <button type="button" className={styles.forget} onClick={() => void forget()}>
                Forget these
              </button>
            </div>
          )}

          {typed && rows.length > 0 && results !== null && (
            <p className={styles.count}>{foundSaying(results)}</p>
          )}

          <div className={styles.list} role="listbox" id={listId} aria-label="Lines found">
            {showRecent &&
              recent.map((before, at) => (
                <div
                  key={before}
                  id={optionId(at)}
                  role="option"
                  aria-selected={at === selected}
                  className={styles.before}
                  onPointerMove={() => {
                    select(at);
                  }}
                  onClick={() => {
                    type(before);
                  }}
                >
                  {before}
                </div>
              ))}

            {typed &&
              rows.map((row, at) => (
                <Fragment key={`${String(row.filmId)}:${String(row.startMs)}:${String(at)}`}>
                  {row.opensFilm && (
                    <p className={styles.group}>
                      {row.title}
                      {row.year !== null && <span className={styles.year}>{row.year}</span>}
                    </p>
                  )}

                  <div
                    id={optionId(at)}
                    role="option"
                    aria-selected={at === selected}
                    className={styles.row}
                    // The pointer moving over a line is somebody considering
                    // it, so Enter should open the one under the cursor rather
                    // than the one the arrows last left behind.
                    onPointerMove={() => {
                      select(at);
                    }}
                    onClick={() => {
                      choose(row);
                    }}
                  >
                    <span className={styles.said}>
                      {row.snippet.map((part, piece) => (
                        <span
                          // Nothing about a piece of a line identifies it but
                          // where it sits, and the line is redrawn whole.
                          key={piece}
                          className={part.matched ? styles.matched : undefined}
                        >
                          {part.text}
                        </span>
                      ))}
                    </span>
                    <span className={styles.at}>{clockOf(row.startMs)}</span>
                  </div>

                  {row.withheld > 0 && (
                    <p className={styles.withheld}>
                      and {row.withheld.toLocaleString('en-GB')} more in this film
                    </p>
                  )}
                </Fragment>
              ))}
          </div>

          {!typed && recent.length === 0 && (
            <p className={styles.nothing}>
              Every line of every subtitle file in your library is here. Type a few words of one and
              it will take you to the moment it is said.
            </p>
          )}

          {typed && rows.length === 0 && problem === null && answered !== '' && !searching && (
            <p className={styles.nothing}>
              {nothingSaying(
                answered,
                scoped === undefined ? null : { title: scoped.title, lines: linesOf(scoped) },
                library,
              )}
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

/**
 * Ctrl+K from anywhere, and Ctrl+Shift+K to look through everything.
 *
 * Bound to the window rather than to anything on screen, which is the point of
 * it. The route is read at the moment the key is pressed rather than subscribed
 * to, so that changing screens does not rebind the listener.
 */
function useOpensAnywhere() {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey) return;
      if (event.key.toLowerCase() !== OPENS) return;
      if (!event.ctrlKey && !event.metaKey) return;

      const { open, show, hide, toggleScope } = useSearch.getState();
      if (open) {
        // Already up, so the plain shortcut puts it away. The widening one is
        // the palette's own business and its handler has already had it.
        if (!event.shiftKey) hide();
        return;
      }

      const route = useNavigation.getState().route;
      // Kept to the film being watched, since that is what the question is
      // usually about, unless the shift key says otherwise.
      const film = route.screen === 'player' ? route.filmId : null;
      show(film);
      if (event.shiftKey && film !== null) toggleScope();

      event.preventDefault();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);
}

/**
 * Puts the palette on the top layer and takes it off again.
 *
 * The platform's own modal dialog rather than an overlay of ours: it traps
 * focus, it takes everything behind it out of the tab order and away from a
 * screen reader, and no stacking context on the page can get above it.
 */
function useShown(
  dialog: RefObject<HTMLDialogElement | null>,
  field: RefObject<HTMLInputElement | null>,
  open: boolean,
) {
  useEffect(() => {
    const element = dialog.current;
    if (element === null) return;

    if (open && !element.open) {
      element.showModal();
      // Named rather than left to the dialog's own guess at what to focus,
      // since every key the palette reads is read from the field.
      field.current?.focus();
    }
    if (!open && element.open) element.close();
  }, [dialog, field, open]);
}

/** Keeps the selected line in view as the arrows walk past the edge of it. */
function useFollowsSelection(list: RefObject<HTMLDivElement | null>, selected: number) {
  useEffect(() => {
    list.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: 'nearest', behavior: 'auto' });
  }, [list, selected]);
}
