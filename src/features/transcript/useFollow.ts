import { useCallback, useEffect, useState } from 'react';
import { NONE } from '@/shared/media/cues';
import { useSettings } from '@/shared/settings/useSettings';

/**
 * Whether the transcript is chasing the film or being read by hand.
 *
 * These are the two things a transcript panel is for, and they pull in opposite
 * directions: one moves the list without being asked, the other is somebody
 * looking for a line four minutes back. A panel that keeps snatching itself
 * away is worse than one that never follows at all.
 *
 * So the rule is that the person wins and is never overruled. Touching the list
 * stops it following, and the only thing that starts it again is asking for it.
 * Nothing here is on a timer: a transcript that resumed on its own after a few
 * quiet seconds would take the line being read away mid-sentence, which is the
 * behaviour this exists to avoid.
 */

export interface Follow {
  /** The list is moving itself to keep the line being spoken in view. */
  following: boolean;
  /** The person is scrolling. Stop chasing the film. */
  release: () => void;
  /** Catch up with the film and carry on following it. */
  resume: () => void;
}

/**
 * @param active The line being spoken, or [`NONE`] during a silence.
 * @param goTo Puts a line in view. Must be stable: it is what this reacts on.
 */
export function useFollow(active: number, goTo: (index: number) => void): Follow {
  // Whether it starts by following is a preference; whether it carries on is
  // not, and nothing below reads the preference again.
  const [following, setFollowing] = useState(
    () => useSettings.getState().settings.transcriptFollow,
  );

  useEffect(() => {
    if (!following || active === NONE) return;
    goTo(active);
    // Following again is a state change like any other, so asking to resume
    // goes to the line being spoken through this and needs nothing of its own.
  }, [following, active, goTo]);

  const release = useCallback(() => {
    setFollowing(false);
  }, []);

  const resume = useCallback(() => {
    setFollowing(true);
  }, []);

  return { following, release, resume };
}
