import type { AccentView } from '@/shared/ipc/bindings';
import { accentFrom } from './colours';
import { centred } from './crop';
import { TILE_RATIO } from './grid';

/**
 * Turning a frame into a poster, off the main thread.
 *
 * The frame arrives already decoded, as a bitmap the main thread transferred
 * rather than copied. Everything expensive happens here: the crop, reading a
 * few hundred thousand pixels back to find what colour the film is, and the
 * WebP encode, which is the slowest part of a capture and would show as
 * dropped frames in the grid if it ran anywhere else.
 */

/** How wide a poster is stored: twice the tile, so it holds up on a dense display. */
const WIDTH = 420;

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
  const height = Math.round(WIDTH * TILE_RATIO);
  const canvas = new OffscreenCanvas(WIDTH, height);
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('this machine will not draw a poster');

  const crop = centred(bitmap.width, bitmap.height, WIDTH / height);
  if (crop.width === 0) throw new Error('the frame came back with no size');
  context.drawImage(bitmap, crop.x, crop.y, crop.width, crop.height, 0, 0, WIDTH, height);

  // Read before the encode rather than after, so the colours come from the
  // frame itself and not from what a lossy encoder made of it.
  const accent = accentFrom(context.getImageData(0, 0, WIDTH, height).data);

  const encoded = await canvas.convertToBlob({ type: 'image/webp', quality: 0.82 });
  return { image: await encoded.arrayBuffer(), accent };
}
