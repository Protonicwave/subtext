import { useMemo } from 'react';
import type { CueView } from '@/shared/ipc/bindings';
import { sourceOf } from '@/shared/media/source';
import { useLibrary } from '@/features/library/useLibrary';
import { Subtitles } from '@/features/player/Subtitles';
import { Choice, Slider } from './controls';
import { SUBTITLE_COLOURS, appearanceOf } from '@/shared/settings/schema';
import { useSettings } from '@/shared/settings/useSettings';
import styles from './SubtitleSection.module.css';

/**
 * What a subtitle looks like, shown as one.
 *
 * The preview is the renderer the player uses, given the settings the player
 * would be given, inside a container sized the way the picture is. It is not a
 * drawing of a subtitle: everything from the size in units of picture height to
 * the way a two line cue stacks is the real thing, because a preview that
 * approximates is a preview that lies about the one case somebody is checking.
 */

/** The line the preview says. Two lines, since that is what stacking is for. */
const SAMPLE: CueView = {
  index: 1,
  startMs: 0,
  endMs: 0,
  text: 'I have never been anywhere the light was quite like this.\nYou get used to it, apparently.',
  position: null,
};

export function SubtitleSection() {
  // The settings themselves rather than the shape made from them: a selector
  // building an object hands back a different one every time it is asked, and
  // the store would read that as a change on every render.
  const settings = useSettings((state) => state.settings);
  const appearance = useMemo(() => appearanceOf(settings), [settings]);

  // A frame from the collection, so the text is being judged over a picture
  // rather than over a flat rectangle. Any film that has one will do.
  const poster = useLibrary((library) => library.films.find((film) => film.posterPath !== null));
  const posterPath = poster?.posterPath ?? null;
  const backdrop = useMemo(
    () => (posterPath === null ? undefined : sourceOf(posterPath)),
    [posterPath],
  );

  return (
    <>
      <div
        className={styles.preview}
        style={backdrop === undefined ? undefined : { backgroundImage: `url("${backdrop}")` }}
      >
        <Subtitles cue={SAMPLE} appearance={appearance} lifted={false} />
      </div>

      <Choice
        name="subtitleTypeface"
        label="Typeface"
        hint="The same two the rest of the application is set in."
        options={[
          { value: 'sans', label: 'Sans' },
          { value: 'serif', label: 'Serif' },
        ]}
      />

      <Slider
        name="subtitleSize"
        label="Size"
        hint="Measured against the height of the picture, so it holds whether the window is small or the film is filling a television."
        format={(size) => `${size.toFixed(1)}%`}
      />

      <Slider name="subtitleWeight" label="Weight" format={weightNamed} />

      <Choice
        name="subtitleColour"
        label="Colour"
        options={SUBTITLE_COLOURS.map((colour) => ({ value: colour.value, label: colour.name }))}
      />

      <Choice
        name="subtitleBackground"
        label="Behind the text"
        hint="A shadow reads over almost any frame. A panel is for a bright, busy film. None is for somebody who would rather have the picture whole."
        options={[
          { value: 'shadow', label: 'Shadow' },
          { value: 'panel', label: 'Panel' },
          { value: 'none', label: 'Nothing' },
        ]}
      />

      <Slider
        name="subtitlePosition"
        label="Height above the bottom"
        hint="The lines lift further when the controls appear, so nothing is ever half hidden behind the scrubber."
        format={(position) => `${position.toFixed(1)}%`}
      />
    </>
  );
}

function weightNamed(weight: number): string {
  if (weight <= 400) return 'Regular';
  if (weight <= 500) return 'Medium';
  if (weight <= 600) return 'Semibold';
  return 'Bold';
}
