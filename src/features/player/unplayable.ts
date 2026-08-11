/**
 * Why a film will not play, said in a way somebody can act on.
 *
 * The webview decodes what it decodes. H.264 in an MP4 plays everywhere; HEVC,
 * AV1 and DTS audio frequently do not, and a file carrying them shows a black
 * rectangle and no explanation. Subtext converts nothing, so the honest answer
 * is to name the likely cause and say what would play instead.
 *
 * The cause is a guess and is written as one. What the element reports is a
 * number between one and four, which says at which point it gave up and not
 * what was wrong with the file, so the container is used to make the guess a
 * useful one.
 */

/** The codes a media element reports, which are numbers and nothing else. */
const ABORTED = 1;
const NETWORK = 2;
const DECODE = 3;
const UNSUPPORTED = 4;

export function whyUnplayable(code: number | null | undefined, path: string): string {
  switch (code) {
    case ABORTED:
      return 'Loading this film was stopped before it could play.';
    case NETWORK:
      return 'This file could not be read. If it sits on an external drive or a network share, it may have been disconnected.';
    case DECODE:
      return `This file opened but its picture or sound could not be decoded. ${likely(path)}`;
    case UNSUPPORTED:
      return `This window cannot open this file. ${likely(path)}`;
    default:
      return 'This film could not be played, and the window did not say why.';
  }
}

/**
 * The usual reason a file of this kind will not play here.
 *
 * Every one of these ends the same way, because the answer is always the same:
 * a copy in H.264 will play, and Subtext will not make one for you.
 */
function likely(path: string): string {
  const advice =
    'Subtext does not convert films. A copy encoded as H.264 with AAC audio will play.';

  switch (extensionOf(path)) {
    case 'mkv':
      return `Matroska files commonly carry HEVC video or DTS audio, and this window can decode neither. ${advice}`;
    case 'avi':
      return `AVI files usually carry an older codec that this window does not decode. ${advice}`;
    case 'mp4':
    case 'm4v':
    case 'mov':
      return `Most files of this kind play, so this one is likely HEVC rather than H.264. ${advice}`;
    case 'webm':
      return `WebM normally plays here, so this file may be damaged or only partly copied. ${advice}`;
    default:
      return `The video inside is in a format this window has no decoder for. ${advice}`;
  }
}

function extensionOf(path: string): string {
  const dot = path.lastIndexOf('.');
  return dot === -1 ? '' : path.slice(dot + 1).toLowerCase();
}
