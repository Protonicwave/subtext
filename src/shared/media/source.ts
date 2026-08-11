import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * The URLs the webview loads files on disk from.
 *
 * Nothing is served over the front end's own origin. Files are handed to the
 * webview through a protocol the shell answers, and each protocol refuses any
 * path that has not been opened to it. This is the only place in the front end
 * that knows how a path becomes a URL.
 *
 * Two protocols, because the two kinds of file are read differently. A poster
 * is a few tens of kilobytes and is read whole, which is what Tauri's own asset
 * protocol is for. A film is gigabytes that must be read a range at a time, and
 * that is ours.
 */

/** The scheme films are served under. The same name appears in `stream.rs`. */
const FILMS = 'stream';

/** A poster or another small file we wrote ourselves. */
export function sourceOf(path: string): string {
  return through(path, 'asset');
}

/** A film, served a range at a time so that seeking does not read the file. */
export function streamOf(path: string): string {
  return through(path, FILMS);
}

function through(path: string, protocol: string): string {
  try {
    return convertFileSrc(path, protocol);
  } catch {
    // Nothing to convert to, because the front end is running somewhere other
    // than inside the shell. An empty source loads nothing, which is the right
    // answer: outside the shell there is no file to load.
    return '';
  }
}
