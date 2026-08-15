import { Choice, Slider } from './controls';

/**
 * How the window looks, which is mostly how much of the film is let into it.
 *
 * The accent, the wash behind everything and the grain over it are one idea in
 * three parts: a dark interface that takes its colour from what is being
 * watched. Each part can be turned down separately, because the amount of it
 * anybody wants depends on the screen they are sitting in front of.
 */
export function AppearanceSection() {
  return (
    <>
      <Choice
        name="accent"
        label="Accent colour"
        hint="Taken from the frame Subtext captured for each film's poster, so the window shifts as you move between them."
        options={[
          { value: 'film', label: 'From the film' },
          { value: 'fixed', label: 'Always amber' },
        ]}
      />

      <Slider
        name="glow"
        label="Ambient light"
        hint="Two wide pools of the accent behind everything, which is what stops a dark window reading as a black rectangle."
        format={(glow) => (glow === 0 ? 'None' : `${String(Math.round(glow * 100))}%`)}
      />

      <Slider
        name="grain"
        label="Grain"
        hint="A faint texture over the whole window. Without some of it the wash above bands into visible stripes on an eight bit display."
        format={(grain) => (grain === 0 ? 'None' : `${String(Math.round((grain / 0.09) * 100))}%`)}
      />

      <Choice
        name="motion"
        label="Movement"
        hint="Following the system means whatever this machine has been told about reduced motion, which is where the answer belongs."
        options={[
          { value: 'system', label: 'Follow the system' },
          { value: 'reduced', label: 'Keep it still' },
        ]}
      />
    </>
  );
}
