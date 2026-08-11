import { describe, expect, it } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { passageOf, selectedIn } from './passage';

function cue(startMs: number, text: string): CueView {
  return { index: 1, startMs, endMs: startMs + 2_000, text, position: null };
}

describe('copying a passage out of a transcript', () => {
  it('puts the moment against every line', () => {
    const copied = passageOf([cue(3_852_000, 'I take scores.'), cue(3_856_000, 'What do you do?')]);

    expect(copied).toBe('[1:04:12] I take scores.\n[1:04:16] What do you do?');
  });

  it('joins a line that was broken to fit across the picture', () => {
    // Where those breaks fall is a fact about the width of a film frame, and
    // means nothing in whatever this is being pasted into.
    expect(
      passageOf([cue(60_000, 'Never have anything in your life\nyou cannot walk out on')]),
    ).toBe('[1:00] Never have anything in your life you cannot walk out on');
  });

  it('keeps a cue with nothing in it as the moment it was', () => {
    expect(passageOf([cue(1_000, '  \n ')])).toBe('[0:01]');
  });

  it('copies nothing for nothing', () => {
    expect(passageOf([])).toBe('');
  });
});

describe('which lines a selection covers', () => {
  const cues = [cue(1_000, 'one'), cue(2_000, 'two'), cue(3_000, 'three')];

  /** The rows as the panel draws them: the index is what names the cue. */
  function draw(): HTMLElement {
    const list = document.createElement('div');
    list.innerHTML = cues
      .map((line, at) => `<div data-index="${String(at)}">${line.text}</div>`)
      .join('');
    document.body.append(list);
    return list;
  }

  function select(list: HTMLElement, from: number, to: number): Selection {
    const range = document.createRange();
    range.setStart(list.children[from]?.firstChild ?? list, 0);
    range.setEnd(list.children[to]?.firstChild ?? list, 1);

    const selection = window.getSelection();
    if (selection === null) throw new Error('the document should have a selection');
    selection.removeAllRanges();
    selection.addRange(range);
    return selection;
  }

  it('takes every line the selection reaches, whole', () => {
    const list = draw();
    // Ending one character into the second line still means that line: a
    // passage is copied as the lines it touches and not as the letters.
    const chosen = selectedIn(list, cues, select(list, 0, 1));

    expect(chosen.map((line) => line.text)).toEqual(['one', 'two']);
  });

  it('leaves out the lines beyond either end of it', () => {
    const list = draw();
    const chosen = selectedIn(list, cues, select(list, 1, 1));

    expect(chosen.map((line) => line.text)).toEqual(['two']);
  });

  it('has nothing to copy when nothing is selected', () => {
    const list = draw();
    window.getSelection()?.removeAllRanges();

    expect(selectedIn(list, cues, window.getSelection())).toEqual([]);
    expect(selectedIn(list, cues, null)).toEqual([]);
  });
});
