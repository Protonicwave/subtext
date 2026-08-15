import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { ShortcutSheet } from './ShortcutSheet';

describe('the keys card', () => {
  it('is not there until it is asked for', () => {
    render(<ShortcutSheet />);

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('comes up on the question mark from wherever somebody is', async () => {
    render(<ShortcutSheet />);

    await userEvent.keyboard('?');

    expect(screen.getByRole('dialog', { name: 'Keys' })).toBeInTheDocument();
    expect(screen.getByText('Play or pause. K does the same')).toBeInTheDocument();
  });

  it('goes away again on the same key', async () => {
    render(<ShortcutSheet />);

    await userEvent.keyboard('?');
    await userEvent.keyboard('?');

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('leaves a question mark typed into a field alone', async () => {
    render(
      <>
        <input aria-label="A field" />
        <ShortcutSheet />
      </>,
    );

    await userEvent.click(screen.getByRole('textbox', { name: 'A field' }));
    await userEvent.keyboard('?');

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'A field' })).toHaveValue('?');
  });
});
