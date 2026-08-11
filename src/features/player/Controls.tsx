import { type ChangeEvent, type CSSProperties, type ReactNode } from 'react';
import {
  ExitFullscreenIcon,
  FullscreenIcon,
  MuteIcon,
  NextLineIcon,
  PauseIcon,
  PlayIcon,
  PreviousLineIcon,
  SkipBackIcon,
  SkipForwardIcon,
  TranscriptIcon,
  VolumeIcon,
} from '@/shared/ui/Icon';
import { clockOf, countdownOf } from '@/shared/media/clock';
import { classes } from '@/shared/ui/classes';
import { BANDS } from './density';
import { PLAYBACK } from './defaults';
import type { Stepping } from './useStepping';
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
  /** Moving by line, which a film with no subtitles cannot do. */
  stepping: Stepping;
  /** The dialogue drawn along the scrubber, as a path. */
  density: string;
  visible: boolean;
  fullscreen: boolean;
  /** The transcript is beside the film. */
  transcript: boolean;
  onToggleFullscreen: () => void;
  onToggleTranscript: () => void;
  /** The pointer is resting here, so the bar should not go away under it. */
  onHold: (holding: boolean) => void;
}

export function Controls({
  playback,
  transport,
  stepping,
  density,
  visible,
  fullscreen,
  transcript,
  onToggleFullscreen,
  onToggleTranscript,
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
      <div
        className={styles.track}
        style={{ '--played': played(positionMs, durationMs) } as CSSProperties}
      >
        {/*
         * The film's dialogue, drawn behind the scrubber. Two copies of the
         * same shape: the second is clipped to how much has been played, which
         * is what fills it in as the film runs without redrawing the path.
         */}
        <svg
          className={styles.density}
          viewBox={`0 0 ${String(BANDS)} 100`}
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <path className={styles.quiet} d={density} />
          <path className={styles.spoken} d={density} />
        </svg>

        <input
          type="range"
          className={classes(styles.slider, styles.scrubber)}
          min={0}
          max={durationMs ?? 0}
          step={1000}
          value={Math.min(positionMs, durationMs ?? positionMs)}
          onChange={onScrub}
          disabled={durationMs === null}
          aria-label="Position in the film"
          aria-valuetext={clockOf(positionMs)}
        />
      </div>

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

        {/*
         * Moving by what is said rather than by how long it took to say it.
         * Set apart from the two beside them, because they answer a different
         * question: not how far, but which line.
         */}
        <Control
          label="Previous line"
          onClick={stepping.back}
          disabled={!stepping.available}
          className={styles.stepping}
        >
          <PreviousLineIcon size={15} />
        </Control>

        <Control label="Next line" onClick={stepping.on} disabled={!stepping.available}>
          <NextLineIcon size={15} />
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
          label={transcript ? 'Hide the transcript' : 'Show the transcript'}
          onClick={onToggleTranscript}
          className={transcript ? styles.on : undefined}
        >
          <TranscriptIcon size={17} />
        </Control>

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
  disabled?: boolean | undefined;
  className?: string | undefined;
  children: ReactNode;
}

/**
 * One icon button.
 *
 * The name is on the button rather than in the icon, which is why every icon in
 * the application is hidden from assistive technology.
 */
function Control({ label, onClick, disabled, className, children }: ControlProps) {
  return (
    <button
      type="button"
      className={classes(styles.control, className)}
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      title={label}
    >
      {children}
    </button>
  );
}
