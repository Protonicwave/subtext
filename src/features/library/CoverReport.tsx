import type { FilmView } from '@/shared/ipc/bindings';
import { coverNameOf, coverTallyOf } from './covers';
import { fileNameOf } from './useLibrary';
import { useCover } from './useCover';
import styles from './CoverReport.module.css';

/**
 * Where the library's covers came from, after something has changed them.
 *
 * It states and it does not ask. Artwork found on the disk is evidence of a
 * decision somebody already made, either when they filed the picture beside
 * the film or when they pointed at a folder of posters, and a dialogue whose
 * answer is always yes is ceremony. So each place is named with how many films
 * it covered and one of the pictures, which is what makes the statement
 * something a person can check rather than something they have to believe.
 *
 * What it offers instead of a confirmation is the way to settle the leftovers,
 * which is the same door a tile and the film page open: choose an image for
 * this film. A film that takes one leaves the list as the library is read
 * back, which is the whole of the feedback anybody needs.
 */
export function CoverReport({ films }: { films: FilmView[] }) {
  const { places, leftOver } = coverTallyOf(films);

  return (
    <div className={styles.report}>
      {places.length > 0 && (
        <ul className={styles.places}>
          {places.map((place) => (
            <li key={place.source} className={styles.place}>
              <b className={styles.count}>{place.films}</b>
              <span className={styles.about}>
                <span className={styles.name}>{coverNameOf(place.source)}</span>
                {place.example !== null && (
                  <span className={styles.path} title={place.example} data-selectable>
                    {place.example}
                  </span>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}

      {leftOver.length > 0 && (
        <div className={styles.leftOver}>
          <p className={styles.statement}>
            {leftOver.length === 1
              ? 'One film has no artwork on the disk and is drawn from its own title.'
              : `${String(leftOver.length)} films have no artwork on the disk and are drawn from their own titles.`}{' '}
            Pick a picture for any of them here, or from the film itself later.
          </p>
          <ul className={styles.films}>
            {leftOver.map((film) => (
              <Bare key={film.id} film={film} />
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

/** One film with nothing to show for it, and the way to settle that. */
function Bare({ film }: { film: FilmView }) {
  const cover = useCover(film);

  return (
    <li className={styles.film}>
      <span className={styles.title}>
        {film.title}
        <span className={styles.file} title={film.path} data-selectable>
          {fileNameOf(film.path)}
        </span>
      </span>
      <button
        type="button"
        className={styles.choose}
        disabled={cover.busy}
        onClick={() => {
          cover.choose();
        }}
      >
        {cover.busy ? 'Choosing…' : 'Choose a picture'}
      </button>
      {cover.problem !== null && (
        <span role="status" className={styles.problem}>
          {cover.problem}
        </span>
      )}
    </li>
  );
}
