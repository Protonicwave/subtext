import type { AccentView } from '@/shared/ipc/bindings';
import { accentFrom } from './colours';
import { fitted } from './crop';

/**
 * Turning a picture into a poster, off the main thread.
 *
 * The picture arrives already decoded, as a bitmap the main thread transferred
 * rather than copied. Everything expensive happens here: reading a few hundred
 * thousand pixels back to find what colour the film is, and the WebP encode,
 * which is the slowest part of a capture and would show as dropped frames in
 * the grid if it ran anywhere else.
 *
 * A poster is stored at the shape of the picture it was made from. Artwork is
 * already the shape of a cover, and a frame is sixteen by nine and stays that
 * way: cutting it to the shape of a tile here would throw away half the width
 * of the shot before anything downstream had a say, and every place a frame is
 * drawn now wants the whole of it. The tile letterboxes what it is given.
 *
 * A poster already on the disk keeps the shape it was made at until the film it
 * came from is captured again, which is what happens when the file changes.
 * Nothing is thrown away to hurry that along: the old shape draws correctly
 * everywhere it appears, and a rescan that redrew every poster in a library
 * would cost minutes to change what almost nobody would notice.
 */

/** How wide a poster is stored: twice a tile, so it holds up on a dense display. */
const WIDTH = 420;

/**
 * How tall it may get.
 *
 * A picture more than three times taller than it is wide is not a cover, and
 * any file at all can be dropped on a film, so the canvas is bounded rather
 * than trusted. Past the bound the height leads and the width follows, so the
 * shape is kept whatever the picture turned out to be.
 */
const TALLEST = WIDTH * 3;

export interface PosterRequest {
  bitmap: ImageBitmap;
}

export type PosterReply = { image: ArrayBuffer; accent: AccentView | null } | { problem: string };

/**
 * The worker's own global, named for what it can do here.
 *
 * The DOM and worker type libraries describe the same identifiers differently
 * and cannot both be in one program, so this file states the two members it
 * uses rather than pulling in a second definition of everything.
 */
const scope = self as unknown as {
  onmessage: ((event: MessageEvent<PosterRequest>) => void) | null;
  postMessage: (message: PosterReply, transfer?: Transferable[]) => void;
};

scope.onmessage = (event) => {
  void reply(event.data);
};

async function reply(request: PosterRequest) {
  try {
    const { image, accent } = await poster(request.bitmap);
    // Transferred rather than copied: the bytes are on their way out and
    // nothing in here refers to them afterwards.
    scope.postMessage({ image, accent }, [image]);
  } catch (failure) {
    scope.postMessage({
      problem: failure instanceof Error ? failure.message : String(failure),
    });
  } finally {
    request.bitmap.close();
  }
}

async function poster(bitmap: ImageBitmap) {
  if (bitmap.width <= 0 || bitmap.height <= 0)
    throw new Error('the picture came back with no size');

  const { width, height } = fitted(bitmap, WIDTH, TALLEST);
  const canvas = new OffscreenCanvas(width, height);
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('this machine will not draw a poster');

  context.drawImage(bitmap, 0, 0, width, height);

  // Read before the encode rather than after, so the colours come from the
  // picture itself and not from what a lossy encoder made of it.
  const accent = accentFrom(context.getImageData(0, 0, width, height).data);

  const encoded = await canvas.convertToBlob({ type: 'image/webp', quality: 0.82 });
  return { image: await encoded.arrayBuffer(), accent };
}
