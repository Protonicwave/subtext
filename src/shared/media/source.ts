import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * The URL the webview loads a file on disk from.
 *
 * Files are not served over the front end's own origin: they are handed to the
 * webview through Tauri's asset protocol, which refuses any path that has not
 * been opened to it. This turns a path into the URL for that protocol, and it
 * is the only place in the front end that knows how.
 */
export function sourceOf(path: string): string {
  try {
    return convertFileSrc(path);
  } catch {
    // Nothing to convert to, because the front end is running somewhere other
    // than inside the shell. An empty source loads nothing, which is the right
    // answer: outside the shell there is no file to load.
    return '';
  }
}
