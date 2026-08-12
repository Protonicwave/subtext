import { useEffect, useMemo, useRef, useState } from 'react';
import type { CueView } from '@/shared/ipc/bindings';
import { type Comfort, NONE, activeAt, timelineOf } from '@/shared/media/cues';
import { sourceOf } from '@/shared/media/source';
import { useLibrary } from '@/features/library/useLibrary';
import { Subtitles } from '@/features/player/Subtitles';
import { ANY_LANGUAGE, LANGUAGE_CODES, languageNamed } from '@/shared/media/languages';
import { Action, Choice, Picker, Slider } from './controls';
import { SUBTITLE_COLOURS, appearanceOf, comfortOf } from '@/shared/settings/schema';
import { useSettings } from '@/shared/settings/useSettings';
import { Button } from '@/shared/ui/Button';
import { useStillness } from '@/shared/ui/stillness';
import styles from './SubtitleSection.module.css';

/**
 * What a subtitle looks like, and when it is there, shown as one.
 *
 * The preview is the renderer the player uses, given the settings the player
 * would be given, inside a container sized the way the picture is. It is not a
 * drawing of a subtitle: everything from the size in units of picture height to
 * the way a two line cue stacks is the real thing, because a preview that
 * approximates is a preview that lies about the one case somebody is checking.
 *
 * The timing preferences are shown the same way. They are the two settings here
 * whose effect cannot be seen in a still frame, so the preview plays: the lines
 * below the picture are the timeline the player would be handed, built by the
 * same function from the same values.
 */

/**
 * The dialogue the preview says, as a file would have written it.
 *
 * Two lines in the first cue, since how a cue stacks is something the renderer
 * decides. The second is a quarter of a second long, which is shorter than
 * anybody can read and is therefore the case the minimum duration exists for.
 */
const SPOKEN: readonly CueView[] = [
  {
    index: 1,
    startMs: 600,
    endMs: 3_400,
    text: 'I have never been anywhere the light was quite like this.\nYou get used to it, apparently.',
    position: null,
  },
  { index: 2, startMs: 4_200, endMs: 4_450, text: 'Not really.', position: null },
];

/** How long the preview takes to play through, in milliseconds. */
const CYCLE = 6_200;

/** The line the preview rests on, so that it is a subtitle when it is still. */
const AT_REST = 0;

export function SubtitleSection() {
  return (
    <>
      <Preview />

      <Picker
        name="subtitleLanguage"
        label="Preferred language"
        hint="Which subtitle a film opens on when it was found with more than one and nobody has chosen. A film you have chosen for keeps what you gave it."
        options={[
          { value: ANY_LANGUAGE, label: 'No preference' },
          ...LANGUAGE_CODES.map((code) => ({ value: code, label: languageNamed(code) })),
        ]}
      />

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

      <Slider
        name="subtitleLeadInMs"
        label="Appears before it is spoken"
        hint="Reading is slower than hearing, so a line that arrives on the first syllable reads as late even though it is right. The end of the line is left where it was, so it is held for longer rather than moved."
        format={(lead) => (lead === 0 ? 'As written' : `${String(lead)} ms`)}
      />

      <Slider
        name="subtitleMinimumMs"
        label="Held for at least"
        hint="The shortest time a line stays up, however short it is. A line is never held into the start of the one after it."
        format={heldFor}
      />
    </>
  );
}

/**
 * The frame, and the timing underneath it.
 *
 * Kept apart from the controls so that the frame loop redraws a picture and two
 * rows of bars rather than every control on the panel sixty times a second.
 */
function Preview() {
  // The settings themselves rather than the shape made from them: a selector
  // building an object hands back a different one every time it is asked, and
  // the store would read that as a change on every render.
  const settings = useSettings((state) => state.settings);
  const appearance = useMemo(() => appearanceOf(settings), [settings]);
  const comfort = useMemo(() => comfortOf(settings), [settings]);
  // The timeline the player would be handed for these two lines, built by the
  // function that builds a film's. What is on screen below is what would be.
  const drawn = useMemo(() => timelineOf(SPOKEN, comfort), [comfort]);

  const still = useStillness();
  const [playing, setPlaying] = useState(false);
  const [shown, setShown] = useState(AT_REST);
  const head = useRef<HTMLSpanElement>(null);

  // When it started, and whether it is going. Held here as well as in state
  // because a change arriving mid-play must not send it back to the beginning:
  // dragging a slider is sixty changes a second, and a preview restarted sixty
  // times a second is a preview that never gets as far as the first line.
  const from = useRef(0);
  const running = useRef(false);

  const play = () => {
    if (running.current || still) return;
    running.current = true;
    from.current = performance.now();
    setPlaying(true);
  };

  // Playing on a change is the demonstration: move either control and the line
  // does what the new value says. Not on arrival, though, since a panel that
  // starts moving by itself is a panel nobody asked to be moving.
  const arrived = useRef(true);
  useEffect(() => {
    if (arrived.current) {
      arrived.current = false;
      return;
    }
    play();
    // These two values and nothing else, so that changing the colour does not
    // set it playing.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- see above
  }, [settings.subtitleLeadInMs, settings.subtitleMinimumMs]);

  useEffect(() => {
    if (!playing) return;

    let frame = 0;
    let up = NONE;

    const tick = (now: number) => {
      const at = now - from.current;
      if (at >= CYCLE) {
        running.current = false;
        setPlaying(false);
        setShown(AT_REST);
        return;
      }

      // Written straight onto the element. It moves every frame and nothing
      // else on the panel depends on where it is, so putting it through state
      // would redraw the whole preview to move one line two pixels.
      head.current?.style.setProperty('--at', String(at / CYCLE));

      const line = activeAt(drawn, at);
      if (line !== up) {
        up = line;
        setShown(line);
      }

      frame = requestAnimationFrame(tick);
    };

    frame = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [playing, drawn]);

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
        <Subtitles cue={drawn.cues[shown] ?? null} appearance={appearance} lifted={false} />
      </div>

      <div className={styles.timing} role="img" aria-label={timingSaid(comfort)}>
        <span className={styles.label}>Spoken</span>
        <div className={styles.lane}>
          {SPOKEN.map((cue) => (
            <span key={cue.index} className={styles.said} style={spanOf(cue)} />
          ))}
        </div>

        <span className={styles.label}>On screen</span>
        <div className={styles.lane}>
          {drawn.cues.map((cue) => (
            <span key={cue.index} className={styles.up} style={spanOf(cue)} />
          ))}
        </div>

        {playing && <span ref={head} className={styles.head} />}
      </div>

      {!still && (
        <Action
          label="Play the preview"
          hint="Runs the two lines above at the values below, which is the only way to see what either of them does."
        >
          <Button onClick={play}>Play</Button>
        </Action>
      )}
    </>
  );
}

/** Where a cue sits along the preview, as a share of it. */
function spanOf(cue: CueView) {
  return {
    left: `${((cue.startMs / CYCLE) * 100).toFixed(3)}%`,
    width: `${(((cue.endMs - cue.startMs) / CYCLE) * 100).toFixed(3)}%`,
  };
}

/** The two rows of bars, said in words, for anybody who is not looking at them. */
function timingSaid({ leadInMs, minimumMs }: Comfort): string {
  const lead =
    leadInMs === 0
      ? 'A line appears exactly when it is spoken'
      : `A line appears ${String(leadInMs)} milliseconds before it is spoken`;
  const held =
    minimumMs === 0
      ? 'and is held for as long as the file asks'
      : `and is held for at least ${(minimumMs / 1_000).toFixed(2)} seconds`;

  return `${lead}, ${held}.`;
}

function heldFor(minimumMs: number): string {
  return minimumMs === 0 ? 'As written' : `${(minimumMs / 1_000).toFixed(2)} s`;
}

function weightNamed(weight: number): string {
  if (weight <= 400) return 'Regular';
  if (weight <= 500) return 'Medium';
  if (weight <= 600) return 'Semibold';
  return 'Bold';
}
