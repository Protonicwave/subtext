import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { FilmView } from '@/shared/ipc/bindings';

const { windowControls } = vi.hoisted(() => ({
  windowControls: {
    minimise: vi.fn(() => Promise.resolve()),
    toggleMaximise: vi.fn(() => Promise.resolve()),
    close: vi.fn(() => Promise.resolve()),
    isMaximised: vi.fn(() => Promise.resolve(false)),
    onResized: vi.fn(() => Promise.resolve(() => undefined)),
  },
}));

vi.mock('@/shared/window/controls', () => ({ windowControls }));

const { TitleBar } = await import('./TitleBar');
const { useNavigation } = await import('./routes');
const { useLibrary } = await import('@/features/library/useLibrary');

const heat = {
  id: 7,
  folderId: 1,
  path: '/films/Heat.1995.mkv',
  title: 'Heat',
  year: 1995,
  durationMs: null,
  posterPath: null,
  accent: null,
  missing: false,
  tracks: [],
  chosenTrackId: null,
  subtitlesOff: false,
  position: null,
} satisfies FilmView;

describe('the title bar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    windowControls.isMaximised.mockResolvedValue(false);
    useNavigation.setState({ route: { screen: 'library' }, previous: null });
    useLibrary.setState({ films: [heat], folders: [] });
  });

  it('says which screen the window is on', async () => {
    render(<TitleBar />);

    expect(screen.getByText('Your films')).toBeInTheDocument();

    useNavigation.getState().openFilm(7);

    expect(await screen.findByText('Heat')).toBeInTheDocument();
  });

  it('works the window buttons', async () => {
    render(<TitleBar />);

    await userEvent.click(screen.getByRole('button', { name: 'Minimise' }));
    await userEvent.click(screen.getByRole('button', { name: 'Maximise' }));
    await userEvent.click(screen.getByRole('button', { name: 'Close' }));

    expect(windowControls.minimise).toHaveBeenCalled();
    expect(windowControls.toggleMaximise).toHaveBeenCalled();
    expect(windowControls.close).toHaveBeenCalled();
  });

  it('offers to restore a window that is already maximised', async () => {
    windowControls.isMaximised.mockResolvedValue(true);

    render(<TitleBar />);

    expect(await screen.findByRole('button', { name: 'Restore' })).toBeInTheDocument();
  });

  it('goes to the settings and back again', async () => {
    render(<TitleBar />);

    await userEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(useNavigation.getState().route).toEqual({ screen: 'settings' });

    await userEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(useNavigation.getState().route).toEqual({ screen: 'library' });
  });
});
