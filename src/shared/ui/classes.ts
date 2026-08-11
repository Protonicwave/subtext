/**
 * Joins class names, leaving out the ones that are not there.
 *
 * CSS module lookups are typed as possibly undefined, and a conditional class
 * is often nothing at all, so writing them into a template literal puts the
 * word "undefined" in the markup. This is the one place that has to think
 * about it.
 */
export function classes(...names: (string | false | null | undefined)[]): string {
  return names.filter((name): name is string => Boolean(name)).join(' ');
}
