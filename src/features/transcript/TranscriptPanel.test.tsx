import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { NONE } from '@/shared/media/cues';
import { TranscriptPanel } from './TranscriptPanel';

function cue(startMs: number, text: string): CueView {
  return { index: 1, startMs, endMs: startMs + 2_000, text, position: null };
}

const spoken = [
  cue(60_000, 'I take scores.'),
  cue(64_000, 'What do you do?'),
  cue(69_000, 'I rob banks.'),
];

function show(cues: readonly CueView[] = spoken, active = NONE) {
  const onSeek = vi.fn();
  const onClose = vi.fn();
  render(<TranscriptPanel cues={cues} active={active} onSeek={onSeek} onClose={onClose} />);

  return { onSeek, onClose };
}

/** The rows as they are drawn, in the order they are spoken. */
function drawn(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>('[data-index]'));
}

describe('the transcript panel', () => {
  it('shows the dialogue with the moment each line is said', () => {
    show();

    expect(screen.getByText('I rob banks.')).toBeInTheDocument();
    expect(screen.getByText('1:04')).toBeInTheDocument();
    expect(screen.getByText('3 lines')).toBeInTheDocument();
  });

  it('seeks to a line that is chosen', async () => {
    const { onSeek } = show();

    await userEvent.click(screen.getByText('What do you do?'));

    expect(onSeek).toHaveBeenCalledWith(64_000);
  });

  it('is read with the keyboard as well as with the pointer', async () => {
    const { onSeek } = show();

    const line = screen.getByText('I rob banks.').closest('[data-index]');
    (line as HTMLElement).focus();
    await userEvent.keyboard('{Enter}');

    expect(onSeek).toHaveBeenCalledWith(69_000);
  });

  it('marks the line being spoken', () => {
    show(spoken, 1);

    const marked = document.querySelectorAll('[aria-current="true"]');
    expect(marked).toHaveLength(1);
    expect(marked[0]).toHaveTextContent('What do you do?');
  });

  it('copies a passage with its timecodes attached', () => {
    show();
    const lines = drawn();

    const range = document.createRange();
    range.setStart(lines[0] ?? document.body, 0);
    range.setEnd(lines[1] ?? document.body, 1);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);

    const setData = vi.fn();
    fireEvent.copy(lines[0] ?? document.body, { clipboardData: { setData } });

    expect(setData).toHaveBeenCalledWith(
      'text/plain',
      '[1:00] I take scores.\n[1:04] What do you do?',
    );
  });

  it('stops following once it is scrolled by hand, and offers to catch up', async () => {
    show(spoken, 0);
    expect(screen.queryByRole('button', { name: /follow the film/i })).not.toBeInTheDocument();

    fireEvent.wheel(drawn()[0] ?? document.body, { deltaY: -240 });

    const resume = screen.getByRole('button', { name: /follow the film/i });
    await userEvent.click(resume);

    expect(screen.queryByRole('button', { name: /follow the film/i })).not.toBeInTheDocument();
  });

  it('says why a film without subtitles has no transcript', () => {
    show([]);

    expect(screen.getByText(/no subtitle file was paired/i)).toBeInTheDocument();
    expect(drawn()).toHaveLength(0);
  });

  it('can be put away', async () => {
    const { onClose } = show();

    await userEvent.click(screen.getByRole('button', { name: /hide the transcript/i }));

    expect(onClose).toHaveBeenCalled();
  });

  it('draws a handful of lines for a transcript of five thousand', () => {
    const many = Array.from({ length: 5_000 }, (_, at) => cue(at * 1_400, `Line ${String(at)}`));

    const at = performance.now();
    show(many, 0);
    const took = performance.now() - at;

    // The count is a window's worth and the overscan, and it does not move with
    // the length of the film. That is the property the scrolling target rests
    // on: a line costs the same whether there are ten behind it or five
    // thousand.
    expect(drawn().length).toBeLessThan(60);
    expect(screen.getByText('Line 0')).toBeInTheDocument();
    expect(screen.queryByText('Line 4000')).not.toBeInTheDocument();

    // Slow enough to catch a return to drawing all of them, and loose enough
    // not to fail on a busy machine.
    expect(took).toBeLessThan(2_000);
  });
});
