import type { CSSProperties } from 'react';
import type { CueView } from '@/shared/ipc/bindings';
import { classes } from '@/shared/ui/classes';
import type { SubtitleAppearance } from './defaults';
import styles from './Subtitles.module.css';

/**
 * The subtitles, drawn by us.
 *
 * A track element would put this in the platform's hands, and with it every
 * decision about typeface, size, weight, colour, backing and position, none of
 * which it lets us make. It also draws in a shadow root that cannot be moved,
 * so the lines could not lift clear of the control bar when it appears.
 *
 * Everything about how it looks arrives as values. That is what makes the
 * settings screen possible later: nothing here is decided here.
 */

interface SubtitlesProps {
  cue: CueView | null;
  appearance: SubtitleAppearance;
  /** The controls are showing, so the text sits above where they are. */
  lifted: boolean;
}

export function Subtitles({ cue, appearance, lifted }: SubtitlesProps) {
  if (cue === null) return null;

  // Where the file asked for the line to go, defaulting to along the bottom.
  // A sign at the top of the frame is the usual reason a file asks at all.
  const [vertical = 'bottom', horizontal = 'centre'] = (cue.position ?? 'bottom-centre').split('-');

  return (
    <div
      className={classes(styles.layer, lifted && styles.lifted)}
      data-vertical={vertical}
      data-horizontal={horizontal}
      style={
        {
          '--subtitle-size': appearance.size,
          '--subtitle-weight': appearance.weight,
          '--subtitle-colour': appearance.colour,
          '--subtitle-position': appearance.position,
          '--subtitle-font':
            appearance.typeface === 'serif' ? 'var(--font-reading)' : 'var(--font-sans)',
        } as CSSProperties
      }
    >
      <p className={classes(styles.cue, styles[appearance.background])}>
        {/*
         * One element per line, so that the panel treatment hugs each line
         * rather than boxing the whole cue into a rectangle with a ragged
         * edge inside it.
         */}
        {cue.text.split('\n').map((line, at) => (
          <span key={`${String(cue.index)}:${String(at)}`} className={styles.line}>
            {line}
          </span>
        ))}
      </p>
    </div>
  );
}
