import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * The window buttons a frameless window has to provide itself.
 *
 * Gathered behind one small object rather than called from the title bar
 * directly, so that there is a single place where the application talks to the
 * window, and one thing to stand in for when the title bar is tested.
 */
export const windowControls = {
  minimise: () => getCurrentWindow().minimize(),
  toggleMaximise: () => getCurrentWindow().toggleMaximize(),
  close: () => getCurrentWindow().close(),
  isMaximised: () => getCurrentWindow().isMaximized(),
  /** Calls back whenever the window is resized, which includes being maximised. */
  onResized: (handler: () => void) => getCurrentWindow().onResized(handler),
};
