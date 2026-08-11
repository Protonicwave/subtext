import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FilmView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: { recentSearches: vi.fn(() => Promise.resolve<string[]>([])) },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

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
const { useSearch } = await import('@/features/search/useSearch');

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

  it('opens dialogue search, and says which keys do it without one', async () => {
    render(<TitleBar />);

    const search = screen.getByRole('button', { name: /search dialogue/i });
    expect(search).toHaveTextContent('Ctrl K');

    await userEvent.click(search);

    expect(useSearch.getState().open).toBe(true);
  });

  it('keeps the search to the film being watched when one is', async () => {
    useNavigation.getState().openFilm(7);
    render(<TitleBar />);

    await userEvent.click(screen.getByRole('button', { name: /search dialogue/i }));

    expect(useSearch.getState().scope).toBe(7);
  });
});
