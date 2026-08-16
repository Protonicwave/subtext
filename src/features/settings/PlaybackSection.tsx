import { useChrome } from '@/shared/window/chrome';
import { Choice, Group, Switch } from './controls';
import { SubtitleSection } from './SubtitleSection';

/**
 * How a film behaves once it is open, which is mostly how its subtitles read.
 *
 * The subtitles are here rather than in a section of their own because they are
 * not a sixth thing the application does. They are how a film reads while it is
 * playing, they are only ever judged with a film in front of you, and the
 * preview below is the player's own renderer for exactly that reason.
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

      <Group title="Subtitles" />
      <SubtitleSection />
    </>
  );
}
