import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Controls } from './Controls';
import { shapeOf } from './density';
import { PLAYBACK } from './defaults';
import type { Stepping } from './useStepping';
import type { Playback, Transport } from './usePlayback';

const playing: Playback = {
  ready: true,
  playing: true,
  waiting: false,
  positionMs: 3_852_000,
  durationMs: 10_260_000,
  volume: 0.8,
  muted: false,
  problem: null,
};

function show(playback: Partial<Playback> = {}, visible = true, available = true) {
  const transport: Transport = {
    toggle: vi.fn(),
    positionNow: vi.fn(() => playing.positionMs),
    seekTo: vi.fn(),
    skipBy: vi.fn(),
    setVolume: vi.fn(),
    toggleMute: vi.fn(),
  };
  const stepping: Stepping = { available, back: vi.fn(), on: vi.fn() };

  render(
    <Controls
      playback={{ ...playing, ...playback }}
      transport={transport}
      stepping={stepping}
      density={shapeOf([0.2, 1, 0])}
      visible={visible}
      fullscreen={false}
      transcript={false}
      onToggleFullscreen={vi.fn()}
      onToggleTranscript={vi.fn()}
      onHold={vi.fn()}
    />,
  );

  return { transport, stepping };
}

describe('the control bar', () => {
  it('says how far in the film is and how much is left', () => {
    show();

    expect(screen.getByText('1:04:12')).toBeInTheDocument();
    expect(screen.getByText('-1:46:48')).toBeInTheDocument();
  });

  it('offers to pause what is playing, and to play what is paused', async () => {
    const { transport } = show();
    await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(transport.toggle).toHaveBeenCalled();

    show({ playing: false });
    expect(screen.getByRole('button', { name: 'Play' })).toBeInTheDocument();
  });

  it('skips by the interval it says it does', async () => {
    const { transport } = show();

    await userEvent.click(screen.getByRole('button', { name: /forward 10 seconds/i }));
    expect(transport.skipBy).toHaveBeenCalledWith(PLAYBACK.skipMs);

    await userEvent.click(screen.getByRole('button', { name: /back 10 seconds/i }));
    expect(transport.skipBy).toHaveBeenCalledWith(-PLAYBACK.skipMs);
  });

  it('steps by line as well as by seconds', async () => {
    const { stepping } = show();

    await userEvent.click(screen.getByRole('button', { name: /next line/i }));
    expect(stepping.on).toHaveBeenCalled();

    await userEvent.click(screen.getByRole('button', { name: /previous line/i }));
    expect(stepping.back).toHaveBeenCalled();
  });

  it('has nothing to step through in a film with no subtitles', () => {
    show({}, true, false);

    expect(screen.getByRole('button', { name: /next line/i })).toBeDisabled();
    expect(screen.getByRole('button', { name: /previous line/i })).toBeDisabled();
    // The seconds still work: they are what a film without dialogue has.
    expect(screen.getByRole('button', { name: /forward 10 seconds/i })).toBeEnabled();
  });

  it('seeks to wherever the scrubber is put', () => {
    const { transport } = show();
    const scrubber = screen.getByRole('slider', { name: /position in the film/i });

    expect(scrubber).toHaveValue('3852000');
    expect(scrubber).toHaveAttribute('max', '10260000');

    // A range input reports the position it was moved to, however it was moved.
    fireEvent.change(scrubber, { target: { value: '600000' } });
    expect(transport.seekTo).toHaveBeenCalledWith(600_000);
  });

  it('draws the dialogue of the film along the scrubber', () => {
    show();

    // Two copies of the one shape: the second is clipped to how much has been
    // played, so filling it in as the film runs costs nothing per frame.
    expect(document.querySelectorAll(`path[d="${shapeOf([0.2, 1, 0])}"]`)).toHaveLength(2);
  });

  it('will not offer to seek a film whose length is unknown', () => {
    show({ durationMs: null });

    expect(screen.getByRole('slider', { name: /position in the film/i })).toBeDisabled();
    expect(screen.getByText('--:--')).toBeInTheDocument();
  });

  it('reports the volume as a figure a person would say', () => {
    show({ volume: 0.8 });

    expect(screen.getByRole('slider', { name: 'Volume' })).toHaveAttribute(
      'aria-valuetext',
      '80 per cent',
    );
  });

  it('shows a muted film as silent whatever the volume was', () => {
    show({ muted: true, volume: 0.8 });

    expect(screen.getByRole('slider', { name: 'Volume' })).toHaveValue('0');
    expect(screen.getByRole('button', { name: 'Unmute' })).toBeInTheDocument();
  });

  it('takes the controls out of reach once they have gone', () => {
    show({}, false);

    // Faded out is not enough: a control nobody can see should not be the next
    // thing the Tab key lands on.
    expect(screen.getByRole('button', { name: 'Pause' }).closest('[inert]')).not.toBeNull();
  });
});
