import { create } from 'zustand';
import type { Id } from '@/shared/ipc/bindings';

/**
 * The frame each partly watched film is showing.
 *
 * The continue watching row shows where you stopped rather than the film's
 * poster, which means a second frame taken from a second moment. These are not
 * cached to disk the way posters are: the moment moves every time somebody
 * watches anything, so a cache of them would be mostly stale files, and taking
 * one costs a seek in a file that is already open.
 */

interface Frame {
  /** The film and the moment this was taken at, so a moment that moved is noticed. */
  key: string;
  url: string;
}

interface FrameState {
  frames: Record<Id, Frame | undefined>;
  keep: (id: Id, key: string, image: ArrayBuffer) => void;
  /** Lets go of every frame, which is what closing the window comes to. */
  clear: () => void;
}

export const useFrames = create<FrameState>((set, get) => ({
  frames: {},

  keep: (id, key, image) => {
    const url = URL.createObjectURL(new Blob([image], { type: 'image/webp' }));
    // A blob URL is a reference the document holds until it is told otherwise,
    // and the frame it points at is a few tens of kilobytes.
    release(get().frames[id]);
    set((state) => ({ frames: { ...state.frames, [id]: { key, url } } }));
  },

  clear: () => {
    Object.values(get().frames).forEach(release);
    set({ frames: {} });
  },
}));

/** What a film's frame would be filed under at a given moment. */
export function keyFor(id: Id, positionMs: number): string {
  return `${String(id)}:${String(positionMs)}`;
}

function release(frame: Frame | undefined) {
  if (frame) URL.revokeObjectURL(frame.url);
}
