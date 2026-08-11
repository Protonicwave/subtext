import { act } from '@testing-library/react';

/**
 * A video element that does something.
 *
 * jsdom has the shape of a media element and none of the behaviour: it decodes
 * nothing, so it never reports a running time, playing throws, and no event it
 * would fire is ever fired. Anything testable about the player sits on those
 * four things.
 *
 * What this reproduces is only the part the player talks to: how long the film
 * is, where it is, whether it is playing, and what went wrong. Deliberately not
 * a media element. Buffering, rates, tracks and the readiness ladder are all
 * the platform's, and none of them is what the player decides anything on.
 */

interface Pretending {
  duration: number;
  currentTime: number;
  paused: boolean;
  volume: number;
  muted: boolean;
  code: number | null;
}

const pretending = new WeakMap<HTMLMediaElement, Pretending>();

function stateOf(element: HTMLMediaElement): Pretending {
  let state = pretending.get(element);
  if (state === undefined) {
    state = {
      duration: Number.NaN,
      currentTime: 0,
      paused: true,
      volume: 1,
      muted: false,
      code: null,
    };
    pretending.set(element, state);
  }
  return state;
}

/** Installs the double. Call it once, at the top of a test file. */
export function pretendMediaWorks() {
  property('duration', (state) => state.duration);
  property('paused', (state) => state.paused);
  property('error', (state) => (state.code === null ? null : ({ code: state.code })));

  writable('currentTime');
  writable('volume');
  writable('muted');

  HTMLMediaElement.prototype.play = function play(this: HTMLMediaElement) {
    stateOf(this).paused = false;
    this.dispatchEvent(new Event('play'));
    return Promise.resolve();
  };

  HTMLMediaElement.prototype.pause = function pause(this: HTMLMediaElement) {
    stateOf(this).paused = true;
    this.dispatchEvent(new Event('pause'));
  };

  HTMLMediaElement.prototype.load = function load() {
    // Nothing to load. What a test wants to happen next, it says itself.
  };
}

/** The film has been opened, and turned out to be this long. */
export function opens(video: HTMLVideoElement, durationMs: number) {
  stateOf(video).duration = durationMs / 1000;
  announce(video, 'loadedmetadata');
  announce(video, 'canplay');
}

/** The film is now here, which is what moves the clock in the controls. */
export function reaches(video: HTMLVideoElement, positionMs: number) {
  stateOf(video).currentTime = positionMs / 1000;
  announce(video, 'timeupdate');
}

/** The element gave up, with one of the four codes it can give up with. */
export function refuses(video: HTMLVideoElement, code: number) {
  stateOf(video).code = code;
  announce(video, 'error');
}

/** Where the film has been put, in milliseconds. */
export function positionOf(video: HTMLVideoElement): number {
  return Math.round(stateOf(video).currentTime * 1000);
}

function announce(video: HTMLVideoElement, event: string) {
  act(() => {
    video.dispatchEvent(new Event(event));
  });
}

function property(name: string, read: (state: Pretending) => unknown) {
  Object.defineProperty(HTMLMediaElement.prototype, name, {
    configurable: true,
    get(this: HTMLMediaElement) {
      return read(stateOf(this));
    },
  });
}

function writable(name: 'currentTime' | 'volume' | 'muted') {
  Object.defineProperty(HTMLMediaElement.prototype, name, {
    configurable: true,
    get(this: HTMLMediaElement) {
      return stateOf(this)[name];
    },
    set(this: HTMLMediaElement, value: never) {
      stateOf(this)[name] = value;
      this.dispatchEvent(new Event(name === 'currentTime' ? 'seeked' : 'volumechange'));
    },
  });
}
