import styles from './ProgressRing.module.css';

const RADIUS = 17;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

interface ProgressRingProps {
  /** How far along, from zero to one. Anything outside that is clamped. */
  fraction: number;
  /** What the ring is measuring, for anyone who cannot see it. */
  label: string;
  size?: number;
}

/**
 * A ring that fills as something finishes.
 *
 * Reported as a progress bar rather than drawn silently, so that the indexing
 * screen says how far along it is whether it is being looked at or listened to.
 */
export function ProgressRing({ fraction, label, size = 52 }: ProgressRingProps) {
  const done = Math.min(1, Math.max(0, fraction));
  const percentage = Math.round(done * 100);

  return (
    <svg
      className={styles.ring}
      width={size}
      height={size}
      viewBox="0 0 40 40"
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percentage}
    >
      <circle className={styles.track} cx="20" cy="20" r={RADIUS} />
      <circle
        className={styles.fill}
        cx="20"
        cy="20"
        r={RADIUS}
        strokeDasharray={CIRCUMFERENCE}
        strokeDashoffset={CIRCUMFERENCE * (1 - done)}
        transform="rotate(-90 20 20)"
      />
    </svg>
  );
}
