import { describe, expect, it } from 'vitest';
import type { FilmView, VideoView } from '@/shared/ipc/bindings';
import { resolutionOf, runtimeOf } from './runtime';

/** A film whose picture is a given size, with nothing else filled in. */
function shot(video: Partial<VideoView> | null): FilmView {
  const details =
    video === null
      ? null
      : {
          container: 'mkv',
          sizeBytes: 0,
          averageBitrate: null,
          audio: [],
          video: {
            codec: 'V_MPEG4/ISO/AVC',
            width: null,
            height: null,
            bitDepth: null,
            frameRate: null,
            ...video,
          },
        };

  return { details } as FilmView;
}

describe('how long a film runs', () => {
  it('says hours and minutes once there is an hour of it', () => {
    expect(runtimeOf(170 * 60_000)).toBe('2 hr 50 min');
    expect(runtimeOf(120 * 60_000)).toBe('2 hr');
    expect(runtimeOf(97 * 60_000)).toBe('1 hr 37 min');
    expect(runtimeOf(44 * 60_000)).toBe('44 min');
  });

  it('says nothing about a film nobody has opened', () => {
    expect(runtimeOf(null)).toBeNull();
    // A running time of nothing at all, which a file that would not decode
    // reports rather than refusing outright.
    expect(runtimeOf(0)).toBeNull();
  });
});

describe('what a picture is called', () => {
  it('names it from the width, whatever the film was cropped to', () => {
    expect(resolutionOf(shot({ width: 3840, height: 2160 }))).toBe('2160p');
    // The same disc, in scope. Calling this 1600p would be wrong in a way
    // nobody says aloud.
    expect(resolutionOf(shot({ width: 3840, height: 1600 }))).toBe('2160p');
    expect(resolutionOf(shot({ width: 1920, height: 804 }))).toBe('1080p');
    expect(resolutionOf(shot({ width: 1280, height: 720 }))).toBe('720p');
  });

  it('allows for an encode cropped by a few pixels', () => {
    expect(resolutionOf(shot({ width: 1916, height: 1076 }))).toBe('1080p');
    expect(resolutionOf(shot({ width: 3808, height: 2140 }))).toBe('2160p');
  });

  it('falls back to the height where the width says nothing useful', () => {
    expect(resolutionOf(shot({ width: null, height: 576 }))).toBe('576p');
    expect(resolutionOf(shot({ width: 640, height: 480 }))).toBe('480p');
  });

  it('says nothing about a file this build has not read', () => {
    expect(resolutionOf(shot(null))).toBeNull();
    expect(resolutionOf(shot({ width: null, height: null }))).toBeNull();
  });
});
