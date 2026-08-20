import type { CoverSourceView } from '@/shared/ipc/bindings';

/**
 * What to say about where a film's picture came from.
 *
 * The back end settles which of the six it is and this says it in words. There
 * is one wording, used by the film page and by the mark on a tile, so that the
 * two cannot come to describe the same film differently.
 *
 * Each sentence names the claim rather than the mechanism. Somebody looking at
 * a tile wants to know whether the picture was chosen or guessed at, not which
 * folder was looked in or what a sidecar file is.
 */

/** The short label, which is what fits on a tile. */
const NAMES: Record<CoverSourceView, string> = {
  chosen: 'Your choice',
  'in-file': 'Inside the film',
  beside: 'Beside the film',
  sidecar: 'From a media manager',
  'folder-above': 'From the folder above',
  none: 'From the film itself',
};

/** The longer statement, which is what the film page has room for. */
const STATEMENTS: Record<CoverSourceView, string> = {
  chosen: 'A picture you picked for this film. Nothing Subtext finds later will replace it.',
  'in-file': 'Artwork attached inside the film, put there by whoever made the file.',
  beside: 'A picture sitting next to the film on the disk.',
  sidecar: 'The artwork named in the file a media manager wrote beside the film.',
  'folder-above': 'A picture in the folder above, which serves every film filed under it.',
  none: 'No artwork was found anywhere on the disk, so this is drawn from the film itself.',
};

export function coverNameOf(source: CoverSourceView): string {
  return NAMES[source];
}

export function coverStatementOf(source: CoverSourceView): string {
  return STATEMENTS[source];
}

/**
 * Whether there is a choice to undo.
 *
 * Only a chosen cover can be given back, because it is the only one a scan is
 * not already free to decide. Offering to put back a cover the scan chose would
 * be offering to do nothing.
 */
export function isChosen(source: CoverSourceView): boolean {
  return source === 'chosen';
}
