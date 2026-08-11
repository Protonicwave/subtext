import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

afterEach(() => {
  cleanup();
});

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
