import { useChrome } from '@/shared/window/chrome';
import { Choice, Switch } from './controls';

/**
 * How a film behaves once it is open.
 *
 * One question rather than the six that were here. The five that went were
 * intervals: how far a skip moves, how long the controls wait, how much of a
 * film counts as watched. Somebody asked to choose one of those has to guess
 * what they would prefer without a film in front of them, and almost everybody
 * keeps whatever was there.
 */
export function PlaybackSection() {
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
