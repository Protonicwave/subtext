/**
 * The two figures in the control bar.
 *
 * Both are drawn in tabular figures, so the width of the readout does not
 * change as the digits do. A clock that shuffles the controls sideways once a
 * second is the sort of detail nobody names and everybody notices.
 */

/**
 * A position, written the way a film is talked about.
 *
 * Hours only once there are any: `4:12` for a short, `1:04:12` for a feature.
 * Minutes and seconds are always two digits so the columns line up.
 */
export function clockOf(ms: number): string {
  const total = Math.floor(sane(ms) / 1000);
  const seconds = total % 60;
  const minutes = Math.floor(total / 60) % 60;
  const hours = Math.floor(total / 3600);

  const tail = `${pad(minutes, hours > 0)}:${pad(seconds, true)}`;
  return hours > 0 ? `${String(hours)}:${tail}` : tail;
}

/**
 * How much is left, as a countdown.
 *
 * Written with a leading minus, which is what tells it apart from the elapsed
 * figure beside it at a glance. Rounded up, so that it reads `0:01` for the
 * whole of the last second rather than reaching zero before the film ends.
 */
export function countdownOf(positionMs: number, durationMs: number | null): string {
  if (durationMs === null) return '--:--';

  const left = Math.max(0, sane(durationMs) - sane(positionMs));
  return `-${clockOf(Math.ceil(left / 1000) * 1000)}`;
}

/**
 * A media element reports no running time as `NaN` and a live stream as
 * `Infinity`, and both would otherwise be drawn as digits.
 */
function sane(ms: number): number {
  return Number.isFinite(ms) && ms > 0 ? ms : 0;
}

function pad(value: number, always: boolean): string {
  return always && value < 10 ? `0${String(value)}` : String(value);
}
