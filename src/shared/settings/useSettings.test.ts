import { beforeEach, describe, expect, it, vi } from 'vitest';
import type * as ClientModule from '@/shared/ipc/client';

const { ipc } = vi.hoisted(() => ({
  ipc: {
    readPreferences: vi.fn(),
    writePreference: vi.fn(),
    forgetPreferences: vi.fn(),
  },
}));

vi.mock('@/shared/ipc/client', async () => {
  const actual = await vi.importActual<typeof ClientModule>('@/shared/ipc/client');
  return { ...actual, ipc };
});

const { DEFAULTS } = await import('./schema');
const { appearanceOf } = await import('./schema');
const { useSettings } = await import('./useSettings');

describe('the settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettings.setState({ settings: DEFAULTS, problem: null });
    ipc.readPreferences.mockResolvedValue([]);
    ipc.writePreference.mockResolvedValue(null);
    ipc.forgetPreferences.mockResolvedValue(null);
  });

  it('reads what was kept from the last time', async () => {
    ipc.readPreferences.mockResolvedValue([{ key: 'subtitles.size', value: '6.2' }]);

    await useSettings.getState().load();

    expect(useSettings.getState().settings.subtitleSize).toBe(6.2);
  });

  it('drops the keys an older release left behind, and keeps the rest', async () => {
    ipc.readPreferences.mockResolvedValue([
      { key: 'subtitles.size', value: '6.2' },
      { key: 'transcript.typeface', value: 'serif' },
      { key: 'playback.skip', value: '30000' },
    ]);

    await useSettings.getState().load();

    expect(useSettings.getState().settings.subtitleSize).toBe(6.2);
    await vi.waitFor(() => {
      expect(ipc.forgetPreferences).toHaveBeenCalledWith(['transcript.typeface', 'playback.skip']);
    });
  });

  it('asks for nothing to be forgotten where there is nothing to forget', async () => {
    ipc.readPreferences.mockResolvedValue([{ key: 'subtitles.size', value: '6.2' }]);

    await useSettings.getState().load();

    expect(ipc.forgetPreferences).not.toHaveBeenCalled();
  });

  /*
   * The settings are already on the screen by then, and a library that would
   * not let go of a row nothing reads is still a library everything else can
   * be drawn from.
   */
  it('carries on when the dead keys cannot be dropped', async () => {
    ipc.readPreferences.mockResolvedValue([{ key: 'transcript.follow', value: 'false' }]);
    ipc.forgetPreferences.mockRejectedValue(new Error('the library file is read only'));

    await useSettings.getState().load();

    expect(useSettings.getState().problem).toBeNull();
  });

  it('carries on with the defaults when the library cannot be read', async () => {
    ipc.readPreferences.mockRejectedValue(new Error('the library database refused the request'));

    await useSettings.getState().load();

    expect(useSettings.getState().settings).toEqual(DEFAULTS);
    expect(useSettings.getState().problem).toContain('refused');
  });

  it('applies a change at once and keeps it behind that', () => {
    useSettings.getState().change('subtitleSize', 5);

    expect(useSettings.getState().settings.subtitleSize).toBe(5);
    expect(ipc.writePreference).toHaveBeenCalledWith('subtitles.size', '5');
  });

  it('writes nothing for a change that changes nothing', () => {
    useSettings.getState().change('motion', DEFAULTS.motion);

    expect(ipc.writePreference).not.toHaveBeenCalled();
  });

  it('says so when a setting could not be kept', async () => {
    ipc.writePreference.mockRejectedValue(new Error('the library file is read only'));

    useSettings.getState().change('grain', 0);
    await vi.waitFor(() => {
      expect(useSettings.getState().problem).toContain('read only');
    });

    // What is on screen is what this session is using, whatever the next one
    // will start with.
    expect(useSettings.getState().settings.grain).toBe(0);
  });

  it('hands the renderer the subtitle appearance it takes', () => {
    const appearance = appearanceOf({ ...DEFAULTS, subtitleTypeface: 'serif', subtitleSize: 5.5 });

    expect(appearance).toEqual({
      typeface: 'serif',
      size: 5.5,
      weight: DEFAULTS.subtitleWeight,
      colour: DEFAULTS.subtitleColour,
      background: DEFAULTS.subtitleBackground,
      position: DEFAULTS.subtitlePosition,
    });
  });
});
