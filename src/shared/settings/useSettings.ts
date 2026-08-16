import { create } from 'zustand';
import { ipc, reasonFor } from '@/shared/ipc/client';
import {
  DEFAULTS,
  deadKeysIn,
  type SettingName,
  type Settings,
  settingsFrom,
  storedAs,
} from './schema';

/**
 * What the application has been told to prefer.
 *
 * Read once when the window opens and held here, because almost everything on
 * every screen asks for something in it: the player for what a subtitle looks
 * like, the library for what to do with a film whose file has gone, the window
 * itself for how much light to put behind it.
 *
 * A change is applied here and written behind it, rather than applied once the
 * write comes back. The value is already known, the row is one row, and a
 * slider that only moved after a round trip to a database would be a slider
 * that stuttered. A write that fails says so, and what is on the screen is then
 * what this session is using and not what the next one will start with.
 */
interface SettingsState {
  readonly settings: Settings;
  readonly problem: string | null;
  readonly load: () => Promise<void>;
  readonly change: <Name extends SettingName>(name: Name, value: Settings[Name]) => void;
  readonly clearProblem: () => void;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: DEFAULTS,
  problem: null,

  load: async () => {
    try {
      const stored = await ipc.readPreferences();
      set({ settings: settingsFrom(stored), problem: null });

      // Behind the read rather than before it. Nothing on any screen waits on
      // this, and a library that would not let go of a row nothing reads is
      // still a library everything else can be drawn from.
      const dead = deadKeysIn(stored);
      if (dead.length > 0) void ipc.forgetPreferences(dead).catch(() => undefined);
    } catch (failure) {
      // The defaults are a working application, so this is worth saying and
      // not worth stopping for. Nothing waits on the read for the same reason:
      // every screen is drawn from live values, and the ones it starts with are
      // the ones it would have been given anyway.
      set({ problem: reasonFor(failure) });
    }
  },

  change: (name, value) => {
    if (get().settings[name] === value) return;
    set((state) => ({ settings: { ...state.settings, [name]: value }, problem: null }));

    const { key, value: text } = storedAs(name, value);
    void ipc.writePreference(key, text).catch((failure: unknown) => {
      set({ problem: `That setting could not be kept: ${reasonFor(failure)}` });
    });
  },

  clearProblem: () => {
    set({ problem: null });
  },
}));

/** One setting, for a component that wants a single value to redraw on. */
export function useSetting<Name extends SettingName>(name: Name): Settings[Name] {
  return useSettings((state) => state.settings[name]);
}
