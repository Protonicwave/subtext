import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

afterEach(() => {
  cleanup();
});

/*
 * jsdom lays nothing out and has no ResizeObserver for anything that wants to
 * be told when that changed. A component that measures itself therefore never
 * hears anything at all.
 */
if (typeof ResizeObserver === 'undefined') {
  // Nothing is ever laid out, so nothing ever changes size and the callback is
  // never owed anything.
  const silent = () => undefined;
  globalThis.ResizeObserver = class {
    observe = silent;
    unobserve = silent;
    disconnect = silent;
  };
}

/*
 * Every element in jsdom reports a box of zero by zero, because nothing is ever
 * laid out. A component that virtualises a list reads that as nothing being in
 * view and draws nothing at all, which would leave the poster grid untestable.
 *
 * Reporting the window's own size is the smallest thing that can be said
 * instead. It is not a layout engine, and no test here asserts a measurement:
 * what they assert is which films ended up on the screen.
 */
for (const [measure, from] of [
  ['offsetWidth', 'innerWidth'],
  ['clientWidth', 'innerWidth'],
  ['offsetHeight', 'innerHeight'],
  ['clientHeight', 'innerHeight'],
] as const) {
  Object.defineProperty(HTMLElement.prototype, measure, {
    configurable: true,
    get: () => window[from],
  });
}

/*
 * Nothing scrolls in jsdom either, and the method a component asks with is not
 * there at all rather than doing nothing. Keeping a chosen line in view is
 * scrolling, and what the tests check is which line was chosen.
 */
if (typeof Element.prototype.scrollIntoView !== 'function') {
  Element.prototype.scrollIntoView = () => undefined;
}

/*
 * jsdom does not implement the modal dialog methods, so a component built on
 * the platform's own dialog would render nothing under test.
 *
 * The shim does what the two methods do that the tests care about: the element
 * opens, it closes, and closing announces itself. What it deliberately does not
 * reproduce is the top layer, the focus trap and inertness, which is exactly
 * the part being left to the platform rather than written here, and so is not
 * the part these tests are checking.
 */
const dialogs = HTMLDialogElement.prototype as Partial<HTMLDialogElement>;
if (typeof dialogs.showModal !== 'function') {
  HTMLDialogElement.prototype.showModal = function showModal(this: HTMLDialogElement) {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function close(this: HTMLDialogElement) {
    this.open = false;
    this.dispatchEvent(new Event('close'));
  };
}
