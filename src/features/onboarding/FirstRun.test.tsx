import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FirstRun } from './FirstRun';
import { useImport } from './useImport';

describe('the first run screen', () => {
  const chooseFolder = vi.fn(() => Promise.resolve());

  beforeEach(() => {
    chooseFolder.mockClear();
    useImport.setState({ chooseFolder });
  });

  it('says that nothing is uploaded before it asks for anything', () => {
    render(<FirstRun />);

    expect(
      screen.getByText(/nothing is uploaded and nothing leaves your computer/i),
    ).toBeInTheDocument();
  });

  it('opens the folder picker', async () => {
    render(<FirstRun />);

    await userEvent.click(screen.getByRole('button', { name: /choose your films folder/i }));

    expect(chooseFolder).toHaveBeenCalled();
  });

  it('can be worked from the keyboard alone', async () => {
    render(<FirstRun />);

    await userEvent.tab();
    expect(screen.getByRole('button', { name: /choose your films folder/i })).toHaveFocus();

    await userEvent.keyboard('{Enter}');
    expect(chooseFolder).toHaveBeenCalled();
  });

  it('mentions dropping files as the other way in', () => {
    render(<FirstRun />);

    expect(screen.getByText(/drop files anywhere in the window/i)).toBeInTheDocument();
  });
});
