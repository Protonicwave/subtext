import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { CueView } from '@/shared/ipc/bindings';
import { Subtitles } from './Subtitles';
import { SUBTITLES } from './defaults';

function cue(text: string, position: CueView['position'] = null): CueView {
  return { index: 4, startMs: 1_000, endMs: 3_000, text, position };
}

function draw(spoken: CueView | null, appearance = SUBTITLES, lifted = false) {
  const { container } = render(<Subtitles cue={spoken} appearance={appearance} lifted={lifted} />);
  return container.firstElementChild;
}

describe('the subtitles', () => {
  it('draws the line being spoken', () => {
    draw(cue('I do what I do best.'));

    expect(screen.getByText('I do what I do best.')).toBeInTheDocument();
  });

  it('keeps a line break as two lines rather than one long one', () => {
    draw(cue('I do what I do best.\nI take scores.'));

    expect(screen.getByText('I do what I do best.')).toBeInTheDocument();
    expect(screen.getByText('I take scores.')).toBeInTheDocument();
  });

  it('draws nothing at all between lines', () => {
    expect(draw(null)).toBeNull();
  });

  it('puts a line where the file asked for it', () => {
    const layer = draw(cue('A sign on the wall.', 'top-right'));

    expect(layer).toHaveAttribute('data-vertical', 'top');
    expect(layer).toHaveAttribute('data-horizontal', 'right');
  });

  it('puts a line along the bottom when the file said nothing', () => {
    const layer = draw(cue('Spoken.'));

    expect(layer).toHaveAttribute('data-vertical', 'bottom');
    expect(layer).toHaveAttribute('data-horizontal', 'centre');
  });

  it('is drawn the way it was told to be, not the way it decided', () => {
    const layer = draw(cue('Spoken.'), {
      ...SUBTITLES,
      size: 9,
      weight: 700,
      colour: '#ffee00',
      position: 12,
    });

    expect(layer).toHaveStyle({
      '--subtitle-size': '9',
      '--subtitle-weight': '700',
      '--subtitle-colour': '#ffee00',
      '--subtitle-position': '12',
    });
  });

  it('lifts clear when the controls are showing', () => {
    const resting = draw(cue('Spoken.'), SUBTITLES, false)?.className ?? '';
    const lifted = draw(cue('Spoken.'), SUBTITLES, true)?.className ?? '';

    expect(lifted).not.toEqual(resting);
    expect(lifted.split(' ').length).toBeGreaterThan(resting.split(' ').length);
  });
});
