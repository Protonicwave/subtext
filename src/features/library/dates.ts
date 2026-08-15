/**
 * A date as a column shows it.
 *
 * The day and the month, and the year as well once the date is not in the
 * current one. A library filled last week reads as a set of days, and one
 * filled over five years says which of them each film arrived in, without a
 * column of identical years for the films that arrived this morning.
 *
 * The moment to measure against is passed in rather than read here, so that
 * what this returns depends only on what it was given.
 */
export function dateOf(millis: number, today = new Date()): string {
  const date = new Date(millis);
  if (Number.isNaN(date.getTime())) return '';

  const day = String(date.getDate());
  const month = MONTHS[date.getMonth()] ?? '';
  const year = date.getFullYear();

  return year === today.getFullYear() ? `${day} ${month}` : `${day} ${month} ${String(year)}`;
}

/**
 * Written out rather than taken from `Intl`, since the whole interface is in
 * one language and a month name that changed with the machine's locale would
 * sit in a column headed in English.
 */
const MONTHS = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
] as const;
