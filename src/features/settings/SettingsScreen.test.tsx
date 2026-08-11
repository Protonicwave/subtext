import { cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';
import type { FolderView } from '@/shared/ipc/bindings';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    windowChrome: vi.fn(() => Promise.resolve({ backdrop: false, switchableDecoding: true })),
    writePreference: vi.fn(() => Promise.resolve(null)),
    readPreferences: vi.fn(() => Promise.resolve([])),
    rescan: vi.fn(() => Promise.resolve(null)),
    rebuildIndex: vi.fn(() => Promise.resolve(null)),
    removeFolder: vi.fn(() => Promise.resolve(true)),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { SettingsScreen } = await import('./SettingsScreen');
const { DEFAULTS } = await import('@/shared/settings/schema');
const { useSettings } = await import('@/shared/settings/useSettings');
const { useLibrary } = await import('@/features/library/useLibrary');
const { useImport } = await import('@/features/onboarding/useImport');

const folder = {
  id: 1,
  path: 'D:\\Films',
  addedAt: 0,
  films: 84,
  watching: true,
} satisfies FolderView;

/** Moves to one of the panels in the rail. */
async function open(name: string) {
  await userEvent.click(screen.getByRole('button', { name }));
  return screen.getByRole('region', { name });
}

describe('the settings screen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettings.setState({ settings: DEFAULTS, problem: null });
    useLibrary.setState({ folders: [folder], films: [], resumable: [], loaded: true });
    useImport.setState({ progress: null, stage: 'idle', asked: false });
  });

  it('opens on the library, showing what is being watched', () => {
    render(<SettingsScreen />);

    expect(screen.getByText('D:\\Films')).toBeInTheDocument();
    expect(screen.getByText(/watching/)).toHaveTextContent('84 films');
  });

  it('shows one panel at a time', async () => {
    render(<SettingsScreen />);

    const playback = await open('Playback');

    expect(within(playback).getByText('Skip interval')).toBeInTheDocument();
    expect(screen.queryByText('Watched folders')).not.toBeInTheDocument();
  });

  it('applies a choice and keeps it', async () => {
    render(<SettingsScreen />);
    await open('Appearance');

    await userEvent.click(screen.getByRole('radio', { name: 'Always amber' }));

    expect(useSettings.getState().settings.accent).toBe('fixed');
    expect(ipc.writePreference).toHaveBeenCalledWith('appearance.accent', 'fixed');
  });

  it('reads the folders again when the pairing preference changes', async () => {
    render(<SettingsScreen />);

    await userEvent.click(screen.getByRole('radio', { name: 'Exact names' }));

    expect(ipc.writePreference).toHaveBeenCalledWith('library.matching', 'exact');
    // Pairings that were made on the old preference are only unmade by a read
    // of the folder they were made in.
    await waitFor(() => {
      expect(ipc.rescan).toHaveBeenCalled();
    });
  });

  it('does not read the folders again for a preference about anything else', async () => {
    render(<SettingsScreen />);

    await userEvent.click(screen.getByRole('radio', { name: 'Hide them' }));

    expect(ipc.rescan).not.toHaveBeenCalled();
  });

  it('asks for the index to be rebuilt, once', async () => {
    render(<SettingsScreen />);

    await userEvent.click(screen.getByRole('button', { name: 'Rebuild' }));

    expect(ipc.rebuildIndex).toHaveBeenCalledTimes(1);
  });

  it('will not ask for a rebuild while the films are already being read', () => {
    useImport.setState({ progress: { filesSeen: 3 } as never });

    render(<SettingsScreen />);

    expect(screen.getByRole('button', { name: 'Reading your films' })).toBeDisabled();
  });

  it('draws the subtitle preview with the renderer the player uses', async () => {
    render(<SettingsScreen />);
    const subtitles = await open('Subtitles');

    expect(within(subtitles).getByText(/the light was quite like this/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('radio', { name: 'Panel' }));

    expect(useSettings.getState().settings.subtitleBackground).toBe('panel');
  });

  it('offers the decoding switch only where turning it off would do something', async () => {
    render(<SettingsScreen />);
    await open('Playback');

    await waitFor(() => {
      expect(screen.getByRole('switch', { name: /graphics card/ })).toBeInTheDocument();
    });

    cleanup();
    ipc.windowChrome.mockResolvedValue({ backdrop: false, switchableDecoding: false });

    render(<SettingsScreen />);
    await open('Playback');
    await waitFor(() => {
      expect(ipc.windowChrome).toHaveBeenCalledTimes(2);
    });

    expect(screen.queryByRole('switch', { name: /graphics card/ })).not.toBeInTheDocument();
  });

  it('lists the keys without anybody having to find them first', async () => {
    render(<SettingsScreen />);
    const shortcuts = await open('Shortcuts');

    expect(within(shortcuts).getByText('Search dialogue, in the film being watched')).toBeVisible();
  });
});
