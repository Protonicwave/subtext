import { useChrome } from '@/shared/window/chrome';
import { Choice, Slider, Switch } from './controls';
import { useSetting } from '@/shared/settings/useSettings';

/**
 * How a film behaves once it is open.
 *
 * Every one of these was a constant the player read before it was a control,
 * which is why none of them needed the player rewriting to become one.
 */
export function PlaybackSection() {
  const resume = useSetting('resume');
  // Only the Windows webview takes a switch for how it decodes video.
  // Everywhere else the decision belongs to the platform, and a control that
  // did nothing would be worse than no control at all.
  const switchable = useChrome().switchableDecoding;

  return (
    <>
      <Choice
        name="resume"
        label="Opening a film again"
        hint="Where you were up to is written down either way, so starting from the beginning does not forget it."
        options={[
          { value: 'carry-on', label: 'Carry on' },
          { value: 'beginning', label: 'From the start' },
        ]}
      />

      {resume === 'carry-on' && (
        <Slider
          name="rewindMs"
          label="Run-up when carrying on"
          hint="Coming back to a film in the middle of a line is disorienting. A few seconds is usually enough to remember where you were."
          format={(ms) => (ms === 0 ? 'None' : `${seconds(ms)}s`)}
        />
      )}

      <Slider
        name="watchedFraction"
        label="Counted as watched"
        hint="How far through a film has to be before Subtext stops offering it as something to carry on with. Short of the end, because almost nobody sits through the credits."
        format={(fraction) => `${String(Math.round(fraction * 100))}% through`}
      />

      <Switch
        name="dialogueArrows"
        label="Arrow keys move by line"
        hint="Somebody pressing the left arrow missed a line, and the line is where they meant to go. A film with no subtitles has nothing to land on and moves by the interval below."
      />

      <Slider
        name="skipMs"
        label="Skip interval"
        hint="What the skip controls move by, and the arrow keys where they are not moving by line."
        format={(ms) => `${seconds(ms)}s`}
      />

      <Slider
        name="hideAfterMs"
        label="Controls hide after"
        hint="Time with the pointer still over a playing film. A paused film keeps its controls whatever this says."
        format={(ms) => `${seconds(ms)}s`}
      />

      {switchable && (
        <Switch
          name="hardwareDecoding"
          label="Decode on the graphics card"
          hint="Faster and cooler where it works. Turn it off for a driver that shows a green or blank picture where the film should be. Takes effect the next time Subtext is opened."
        />
      )}
    </>
  );
}

function seconds(ms: number): string {
  return (ms / 1_000).toFixed(ms % 1_000 === 0 ? 0 : 1);
}
