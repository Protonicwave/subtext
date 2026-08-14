import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SyncPanel } from './SyncPanel';
import { RATES, STEP_MS, type Sync } from './useSync';

function show(state: Partial<Sync> = {}) {
  const sync: Sync = {
    available: true,
    offsetMs: 0,
    rate: 1,
    pending: false,
    nudge: vi.fn(),
    setRate: vi.fn(),
    reset: vi.fn(),
    apply: vi.fn(() => Promise.resolve()),
    ...state,
  };
  const onClose = vi.fn();

  render(<SyncPanel sync={sync} onClose={onClose} />);
  return { sync, onClose };
}

describe('the subtitle timing controls', () => {
  it('says which way the subtitles have been moved and by how much', () => {
    show({ offsetMs: -1_250 });
    expect(screen.getByText('−1.25s')).toBeInTheDocument();
  });

  it('says that a file nobody has touched is as it was written', () => {
    show();
    expect(screen.getByText('as written')).toBeInTheDocument();
  });

  it('moves the subtitles a step at a time, either way', async () => {
    const { sync } = show();

    await userEvent.click(screen.getByRole('button', { name: 'Later' }));
    expect(sync.nudge).toHaveBeenCalledWith(STEP_MS);

    await userEvent.click(screen.getByRole('button', { name: 'Earlier' }));
    expect(sync.nudge).toHaveBeenCalledWith(-STEP_MS);
  });

  it('offers nothing to undo on a file nobody has touched', () => {
    show();
    expect(screen.getByRole('button', { name: 'As written' })).toBeDisabled();
  });

  it('goes back to the file as it was written', async () => {
    const { sync } = show({ offsetMs: 400 });

    await userEvent.click(screen.getByRole('button', { name: 'As written' }));
    expect(sync.reset).toHaveBeenCalled();
  });

  it('offers the conversions that actually occur rather than a slider', async () => {
    const { sync } = show();
    const list = screen.getByRole('combobox', { name: /framerate/i });

    // A ratio between two framerates. Anybody who needs another can type it,
    // and nobody has to find one by dragging.
    const wanted = RATES[1];
    await userEvent.selectOptions(list, String(wanted?.value));
    expect(sync.setRate).toHaveBeenCalledWith(wanted?.value);

    expect(screen.getByRole('option', { name: 'A value of my own' })).toBeInTheDocument();
  });

  it('shows the rate a film is already on', () => {
    show({ rate: RATES[2]?.value ?? 1 });
    expect(screen.getByRole('combobox', { name: /framerate/i })).toHaveValue(
      String(RATES[2]?.value),
    );
  });

  it('takes a value of its own once one is asked for', async () => {
    const { sync } = show();

    await userEvent.selectOptions(screen.getByRole('combobox', { name: /framerate/i }), 'other');
    const field = screen.getByRole('spinbutton', { name: /rate/i });
    await userEvent.clear(field);
    await userEvent.type(field, '1.05');
    await userEvent.tab();

    expect(sync.setRate).toHaveBeenCalledWith(1.05);
  });

  it('closes on Escape rather than leaving the film', async () => {
    const { onClose } = show();

    await userEvent.click(screen.getByRole('button', { name: 'Later' }));
    await userEvent.keyboard('{Escape}');

    expect(onClose).toHaveBeenCalled();
  });
});
