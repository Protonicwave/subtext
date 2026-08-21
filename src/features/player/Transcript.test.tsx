import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { NONE } from '@/shared/media/cues';
import { Transcript } from './Transcript';

function cue(startMs: number, text: string): CueView {
  return { index: 1, startMs, endMs: startMs + 2_000, text, position: null };
}

const spoken = [
  cue(60_000, 'I take scores.'),
  cue(64_000, 'What do you do?'),
  cue(69_000, 'I rob banks.'),
];

/** A film's worth of dialogue, for the questions about scrolling. */
const long = Array.from({ length: 400 }, (_, line) => cue(line * 3_000, `Line ${String(line)}`));

function show(cues: readonly CueView[] = spoken, active = NONE) {
  const onSeek = vi.fn();
  const onClose = vi.fn();
  const view = render(<Transcript cues={cues} active={active} onSeek={onSeek} onClose={onClose} />);

  const at = (line: number, lines: readonly CueView[] = cues) => {
    view.rerender(<Transcript cues={lines} active={line} onSeek={onSeek} onClose={onClose} />);
  };

  return { onSeek, onClose, at };
}

/** The rows as they are drawn, in the order they are spoken. */
function drawn(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>('[data-index]'));
}

/** The list, which is what scrolls. */
function scroller(): HTMLElement {
  const list = drawn()[0]?.parentElement?.parentElement;
  if (!list) throw new Error('the lines should sit in a scroller');
  return list;
}

/** The line the film is on, which is the only one marked. */
function marked(): Element | null {
  return document.querySelector('[aria-current="true"]');
}

describe('the dialogue beside the film', () => {
  it('lists the lines with the moment each one is said', () => {
    show();

    expect(screen.getByText('I rob banks.')).toBeInTheDocument();
    expect(screen.getByText('1:04')).toBeInTheDocument();
    expect(screen.getByText('3 lines')).toBeInTheDocument();
  });

  it('marks the line being spoken, and tells the lines either side of it apart', () => {
    show(spoken, 1);

    expect(marked()).toHaveTextContent('What do you do?');
    // What has been said and what is coming, which is one attribute on the
    // rows that changed rather than a style on all of them.
    const [first, second, third] = drawn();
    expect(first).toHaveAttribute('data-said');
    expect(second).not.toHaveAttribute('data-said');
    expect(third).not.toHaveAttribute('data-said');
  });

  it('stays on the last line spoken through a silence', () => {
    // Most of a film is silence. A panel that unmarked itself between every
    // pair of lines would say nothing about where the film had got to.
    const { at } = show(spoken, 1);

    at(NONE);

    expect(marked()).toHaveTextContent('What do you do?');
  });

  it('seeks to a line that is chosen', async () => {
    const { onSeek } = show();

    await userEvent.click(screen.getByText('What do you do?'));

    expect(onSeek).toHaveBeenCalledWith(64_000);
  });

  it('is read with the keyboard as well as with the pointer', async () => {
    const { onSeek } = show();

    drawn()[2]?.focus();
    await userEvent.keyboard('{Enter}');

    expect(onSeek).toHaveBeenCalledWith(69_000);
  });

  it('follows the film, keeping its place in view', () => {
    const { at } = show(long, 0);
    const list = scroller();
    const opened = list.scrollTop;

    at(120);

    expect(list.scrollTop).toBeGreaterThan(opened);
  });

  it('leaves a reader who has scrolled away where they are', () => {
    const { at } = show(long, 0);
    const list = scroller();

    // Somebody looking for a line a long way further on. The film carries on
    // regardless, and the panel must not take the page back off them.
    list.scrollTop = 6_000;
    fireEvent.scroll(list);
    at(1);

    expect(list.scrollTop).toBe(6_000);
  });

  it('picks the film up again once its place is back in view', () => {
    const { at } = show(long, 0);
    const list = scroller();

    list.scrollTop = 6_000;
    fireEvent.scroll(list);
    at(1);
    expect(list.scrollTop).toBe(6_000);

    // Scrolled back to where the film is, which is the whole of what asking to
    // follow it again amounts to.
    list.scrollTop = 0;
    fireEvent.scroll(list);
    at(120);

    expect(list.scrollTop).toBeGreaterThan(0);
  });

  it('takes up the new track when the subtitles are changed part way through', () => {
    const french = [
      cue(60_000, 'Je fais des casses.'),
      cue(64_000, 'Et vous ?'),
      cue(69_000, 'Je braque des banques.'),
    ];
    const { at } = show(spoken, 1);

    at(1, french);

    expect(screen.getByText('Et vous ?')).toBeInTheDocument();
    expect(screen.queryByText('What do you do?')).not.toBeInTheDocument();
    expect(marked()).toHaveTextContent('Et vous ?');
  });

  it('can be put away', async () => {
    const { onClose } = show();

    await userEvent.click(screen.getByRole('button', { name: /hide the dialogue/i }));

    expect(onClose).toHaveBeenCalled();
  });

  it('draws a handful of rows for a track of five thousand lines', () => {
    const many = Array.from({ length: 5_000 }, (_, line) =>
      cue(line * 1_400, `Line ${String(line)}`),
    );

    const started = performance.now();
    show(many, 0);
    const took = performance.now() - started;

    // A window's worth and the overscan, and it does not grow with the length
    // of the film. That is the property the whole panel rests on: a line costs
    // the same whether there are ten behind it or five thousand.
    expect(drawn().length).toBeLessThan(60);
    expect(screen.getByText('Line 0')).toBeInTheDocument();
    expect(screen.queryByText('Line 4000')).not.toBeInTheDocument();

    // Slow enough to catch a return to drawing all of them, and loose enough
    // not to fail on a busy machine.
    expect(took).toBeLessThan(2_000);
  });
});
