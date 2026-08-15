import type { AccentView } from '@/shared/ipc/bindings';
import type { PosterReply, PosterRequest } from './poster.worker';

/**
 * Taking a frame out of a film.
 *
 * The half of the capture that has to be on the main thread, because a video
 * element is the only thing that can decode a film and only a document has one.
 * It does as little as possible: open the file, seek, hand the decoded frame to
 * the worker, and get out of the way.
 *
 * Taking a frame is the last answer rather than the first. A film that carries
 * its own artwork, or has a picture beside it, is drawn from that image
 * instead, which is the same work without the decoder.
 */

/**
 * How far into a film the frame is taken from.
 *
 * Far enough past the distributor logos and the black at the head of the file,
 * and not so far as to be a spoiler. A fifth of the way in is a scene.
 */
const AT = 0.2;

/**
 * Where to look when the file will not say how long it is, which happens with
 * a container whose index is at the end and has not been read yet.
 */
const BLIND = 90;

/** How long any one step is given before the film is treated as unreadable. */
const PATIENCE = 20_000;

export interface Captured {
  /** The encoded frame, as a WebP. */
  image: ArrayBuffer;
  accent: AccentView | null;
  /** How long the film runs, which opening it is the only way to find out. */
  durationMs: number | null;
}

/**
 * Where in the film to take the frame from.
 *
 * A poster is taken a fifth of the way in, wherever that falls. The frame the
 * continue watching row shows is taken from the moment somebody stopped, which
 * is a number of seconds and not a proportion of anything.
 */
export type Moment = { fraction: number } | { seconds: number };

/**
 * The poster drawn from an image that is already on the disk.
 *
 * The bytes are whatever was attached to the film or put beside it, and they go
 * through the same worker a frame does: the same crop to two by three, the same
 * WebP, the same colours read off the pixels. Nothing here opens the film, so
 * this costs a decode of one picture rather than a seek into a four gigabyte
 * file, and it says nothing about how long the film runs because nothing asked.
 */
export async function posterFrom(
  image: Uint8Array<ArrayBuffer>,
  worker: Worker,
): Promise<Captured> {
  const bitmap = await createImageBitmap(new Blob([image]));
  const encoded = await encode(worker, bitmap);

  return { image: encoded.image, accent: encoded.accent, durationMs: null };
}

export async function captureFrom(
  source: string,
  worker: Worker,
  moment: Moment = { fraction: AT },
): Promise<Captured> {
  const video = document.createElement('video');
  video.preload = 'auto';
  video.muted = true;
  // The films are served from a different origin to the window, so without
  // this the frame taints the canvas and its pixels cannot be read back.
  video.crossOrigin = 'anonymous';

  try {
    video.src = source;
    const seconds = await opened(video);
    await seeked(video, momentIn(moment, seconds));

    const encoded = await encode(worker, await createImageBitmap(video));

    return {
      image: encoded.image,
      accent: encoded.accent,
      durationMs: Number.isFinite(seconds) ? Math.round(seconds * 1000) : null,
    };
  } finally {
    // Without this the element holds the file open and keeps whatever it had
    // buffered, which across a library of any size adds up to real memory.
    video.removeAttribute('src');
    video.load();
  }
}

/**
 * The second to seek to, given what the file turned out to be.
 *
 * A container whose index sits at the end of the file can report no running
 * time at all until it has been read, and a proportion of an unknown length is
 * not a moment. Somewhere well inside any feature is the fallback.
 */
export function momentIn(moment: Moment, seconds: number): number {
  if ('seconds' in moment) return moment.seconds;
  return Number.isFinite(seconds) && seconds > 0 ? seconds * moment.fraction : BLIND;
}

/** Waits for the file to say how long it is, or to say it cannot be read. */
function opened(video: HTMLVideoElement): Promise<number> {
  return waitingOn(video, 'loadedmetadata', () => video.duration);
}

/** Moves to a moment and waits for the frame there to be ready to read. */
function seeked(video: HTMLVideoElement, at: number): Promise<void> {
  const moment = Math.max(0, Math.min(at, video.duration || at));
  const arrived = waitingOn(video, 'seeked', () => undefined);
  video.currentTime = moment;
  return arrived;
}

/**
 * One step of the capture, as a promise.
 *
 * Every step can fail the same three ways: the event arrives, the element gives
 * up and fires an error, or neither happens because the file is on a drive that
 * has stopped answering. The last is why there is a timeout at all.
 */
function waitingOn<T>(video: HTMLVideoElement, event: string, taken: () => T): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const done = () => {
      clearTimeout(timer);
      video.removeEventListener(event, onEvent);
      video.removeEventListener('error', onError);
    };
    const onEvent = () => {
      done();
      resolve(taken());
    };
    const onError = () => {
      done();
      reject(new Error(video.error?.message ?? 'this file could not be read'));
    };
    const timer = setTimeout(() => {
      done();
      reject(new Error('this file took too long to answer'));
    }, PATIENCE);

    video.addEventListener(event, onEvent, { once: true });
    video.addEventListener('error', onError, { once: true });
  });
}

/**
 * Hands the frame to the worker and waits for the poster to come back.
 *
 * One request at a time, which is what lets the reply be matched to it by being
 * the next message rather than by carrying an identifier. Captures are run one
 * after another on purpose: they are background work, and a machine busy
 * encoding four posters at once is a machine that scrolls badly.
 */
function encode(
  worker: Worker,
  bitmap: ImageBitmap,
): Promise<{ image: ArrayBuffer; accent: AccentView | null }> {
  return new Promise((resolve, reject) => {
    const done = () => {
      worker.removeEventListener('message', onMessage);
      worker.removeEventListener('error', onError);
    };
    const onMessage = (event: MessageEvent<PosterReply>) => {
      done();
      if ('problem' in event.data) reject(new Error(event.data.problem));
      else resolve(event.data);
    };
    const onError = () => {
      done();
      reject(new Error('the poster could not be encoded'));
    };

    worker.addEventListener('message', onMessage, { once: true });
    worker.addEventListener('error', onError, { once: true });
    worker.postMessage({ bitmap } satisfies PosterRequest, [bitmap]);
  });
}
