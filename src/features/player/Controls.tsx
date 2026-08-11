import { type ChangeEvent, type CSSProperties, type ReactNode } from 'react';
import {
  ExitFullscreenIcon,
  FullscreenIcon,
  MuteIcon,
  PauseIcon,
  PlayIcon,
  SkipBackIcon,
  SkipForwardIcon,
  VolumeIcon,
} from '@/shared/ui/Icon';
import { classes } from '@/shared/ui/classes';
import { clockOf, countdownOf } from './clock';
import { PLAYBACK } from './defaults';
import type { Playback, Transport } from './usePlayback';
import styles from './Controls.module.css';

/**
 * The control bar.
 *
 * The scrubber and the volume are range inputs rather than something drawn from
 * dividers and pointer events. That buys keyboard control, the correct roles
 * and the platform's own drag behaviour for nothing, and the parts of it that
 * are visible are all replaceable in CSS. The density map in the scrubber
 * track is what this becomes later; it is the same element with something drawn
 * behind it.
 */

interface ControlsProps {
  playback: Playback;
  transport: Transport;
  visible: boolean;
  fullscreen: boolean;
  onToggleFullscreen: () => void;
  /** The pointer is resting here, so the bar should not go away under it. */
  onHold: (holding: boolean) => void;
}

export function Controls({
  playback,
  transport,
  visible,
  fullscreen,
  onToggleFullscreen,
  onHold,
}: ControlsProps) {
  const { positionMs, durationMs, playing, volume, muted } = playback;

  const seconds = Math.round(PLAYBACK.skipMs / 1000);
  const onScrub = (event: ChangeEvent<HTMLInputElement>) => {
    transport.seekTo(Number(event.target.value));
  };

  return (
    <div
      className={classes(styles.bar, !visible && styles.gone)}
      // Hidden from everything, not just from the eye. Controls that have
      // faded out should not still be in the tab order.
      inert={!visible}
      onPointerEnter={() => {
        onHold(true);
      }}
      onPointerLeave={() => {
        onHold(false);
      }}
    >
      <input
        type="range"
        className={classes(styles.slider, styles.scrubber)}
        min={0}
        max={durationMs ?? 0}
        step={1000}
        value={Math.min(positionMs, durationMs ?? positionMs)}
        onChange={onScrub}
        disabled={durationMs === null}
        style={{ '--played': played(positionMs, durationMs) } as CSSProperties}
        aria-label="Position in the film"
        aria-valuetext={clockOf(positionMs)}
      />

      <div className={styles.row}>
        <Control
          label={playing ? 'Pause' : 'Play'}
          onClick={transport.toggle}
          className={styles.primary}
        >
          {playing ? <PauseIcon size={19} /> : <PlayIcon size={19} />}
        </Control>

        <Control
          label={`Back ${String(seconds)} seconds`}
          onClick={() => {
            transport.skipBy(-PLAYBACK.skipMs);
          }}
        >
          <SkipBackIcon size={17} />
        </Control>

        <Control
          label={`Forward ${String(seconds)} seconds`}
          onClick={() => {
            transport.skipBy(PLAYBACK.skipMs);
          }}
        >
          <SkipForwardIcon size={17} />
        </Control>

        <p className={styles.clock}>
          <span>{clockOf(positionMs)}</span>
          <span className={styles.left}>{countdownOf(positionMs, durationMs)}</span>
        </p>

        <div className={styles.volume}>
          <Control label={muted ? 'Unmute' : 'Mute'} onClick={transport.toggleMute}>
            {muted || volume === 0 ? <MuteIcon size={17} /> : <VolumeIcon size={17} />}
          </Control>
          <input
            type="range"
            className={classes(styles.slider, styles.level)}
            min={0}
            max={1}
            step={0.02}
            value={muted ? 0 : volume}
            style={{ '--played': muted ? 0 : volume } as CSSProperties}
            onChange={(event) => {
              transport.setVolume(Number(event.target.value));
            }}
            aria-label="Volume"
            aria-valuetext={`${String(Math.round((muted ? 0 : volume) * 100))} per cent`}
          />
        </div>

        <Control
          label={fullscreen ? 'Leave full screen' : 'Full screen'}
          onClick={onToggleFullscreen}
        >
          {fullscreen ? <ExitFullscreenIcon size={17} /> : <FullscreenIcon size={17} />}
        </Control>
      </div>
    </div>
  );
}

/** How much of the film is behind the thumb, from zero to one. */
function played(positionMs: number, durationMs: number | null): number {
  if (durationMs === null || durationMs <= 0) return 0;
  return Math.min(1, positionMs / durationMs);
}

interface ControlProps {
  label: string;
  onClick: () => void;
  className?: string | undefined;
  children: ReactNode;
}

/**
 * One icon button.
 *
 * The name is on the button rather than in the icon, which is why every icon in
 * the application is hidden from assistive technology.
 */
function Control({ label, onClick, className, children }: ControlProps) {
  return (
    <button
      type="button"
      className={classes(styles.control, className)}
      onClick={onClick}
      aria-label={label}
      title={label}
    >
      {children}
    </button>
  );
}
