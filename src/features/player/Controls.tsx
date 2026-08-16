import {
  type ChangeEvent,
  type CSSProperties,
  type PointerEvent,
  type ReactNode,
  useState,
} from 'react';
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
  SubtitlesIcon,
  SyncIcon,
  VolumeIcon,
} from '@/shared/ui/Icon';
import { clockOf, countdownOf } from '@/shared/media/clock';
import { classes } from '@/shared/ui/classes';
import { SKIP_MS } from './intervals';
import { ScrubberPreview } from './ScrubberPreview';
import { SyncPanel } from './SyncPanel';
import { TrackMenu } from './TrackMenu';
import type { Alignment } from './useAlignment';
import type { Stepping } from './useStepping';
import type { Sync } from './useSync';
import type { TrackChoice } from './useTrack';
import type { Playback, Transport } from './usePlayback';
import styles from './Controls.module.css';

/**
 * The control bar.
 *
 * The scrubber and the volume are range inputs rather than something drawn from
 * dividers and pointer events. That buys keyboard control, the correct roles
 * and the platform's own drag behaviour for nothing, and the parts of it that
 * are visible are all replaceable in CSS.
 */

/** What the bar needs to show the moment the pointer is resting over. */
export interface Preview {
  /** The film, through the protocol that serves it. */
  source: string;
  /** What is said around a moment, where anything is. */
  spokenAt: (ms: number) => string | null;
}

interface ControlsProps {
  playback: Playback;
  transport: Transport;
  /** Moving by line, which a film with no subtitles cannot do. */
  stepping: Stepping;
  /** What the bar shows about the moment under the pointer. */
  preview: Preview;
  /** Putting the subtitles back in step with the film. */
  sync: Sync;
  /** Working that out by listening to the film instead. */
  alignment: Alignment;
  /** The timing controls are showing. */
  syncing: boolean;
  /** Which subtitle the film is read with. */
  choice: TrackChoice;
  /** The subtitle menu is showing. */
  choosing: boolean;
  onToggleTracks: () => void;
  visible: boolean;
  fullscreen: boolean;
  onToggleFullscreen: () => void;
  onToggleSync: () => void;
  /** The pointer is resting here, so the bar should not go away under it. */
  onHold: (holding: boolean) => void;
}

export function Controls({
  playback,
  transport,
  stepping,
  preview,
  sync,
  alignment,
  syncing,
  choice,
  choosing,
  visible,
  fullscreen,
  onToggleFullscreen,
  onToggleSync,
  onToggleTracks,
  onHold,
}: ControlsProps) {
  const { positionMs, durationMs, playing, volume, muted } = playback;

  const seconds = Math.round(SKIP_MS / 1000);
  const onScrub = (event: ChangeEvent<HTMLInputElement>) => {
    transport.seekTo(Number(event.target.value));
  };

  // How far along the bar the pointer is, from zero to one, or nothing when it
  // is somewhere else. Kept as a fraction rather than as a moment, because it
  // is what places the preview as well as what names the moment it shows.
  const [along, setAlong] = useState<number | null>(null);

  const onHover = (event: PointerEvent<HTMLDivElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    // A bar of no width has the pointer at the start of it, which is the only
    // moment such a bar could be pointing at.
    const at = box.width > 0 ? (event.clientX - box.left) / box.width : 0;
    setAlong(Math.max(0, Math.min(1, at)));
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
      {/*
       * One at a time, and both on the left of the bar under the buttons that
       * open them. They are about the same thing from either end, and two
       * panels beside each other would be a menu open over a control somebody
       * is still using.
       */}
      {choosing && <TrackMenu choice={choice} onClose={onToggleTracks} />}
      {syncing && !choosing && (
        <SyncPanel sync={sync} alignment={alignment} onClose={onToggleSync} />
      )}

      <div
        className={styles.track}
        style={{ '--played': played(positionMs, durationMs) } as CSSProperties}
        onPointerMove={onHover}
        onPointerLeave={() => {
          setAlong(null);
        }}
      >
        {/*
         * Only while the pointer is over the bar. What draws it opens the film
         * a second time, and holding a decoder on a four gigabyte file to
         * answer a question nobody is asking is not worth the memory.
         */}
        {along !== null && durationMs !== null && (
          <ScrubberPreview
            source={preview.source}
            atMs={along * durationMs}
            along={along}
            line={preview.spokenAt(along * durationMs)}
          />
        )}

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
            transport.skipBy(-SKIP_MS);
          }}
        >
          <SkipBackIcon size={17} />
        </Control>

        <Control
          label={`Forward ${String(seconds)} seconds`}
          onClick={() => {
            transport.skipBy(SKIP_MS);
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

        {/*
         * Only where the film has subtitles at all. One track is still worth
         * the button, since turning it off is one of the answers.
         */}
        <Control
          label={choosing ? 'Hide the subtitle menu' : 'Subtitles'}
          onClick={onToggleTracks}
          disabled={!choice.available}
          className={choosing ? styles.on : undefined}
        >
          <SubtitlesIcon size={17} />
        </Control>

        {/*
         * Only where there is a subtitle to put in step. A film with none has
         * nothing for this to move.
         */}
        <Control
          label={syncing ? 'Hide the subtitle timing' : 'Subtitle timing'}
          onClick={onToggleSync}
          disabled={!sync.available}
          className={syncing ? styles.on : undefined}
        >
          <SyncIcon size={17} />
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
