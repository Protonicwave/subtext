import type { AlignStageView, AlignmentView } from '@/shared/ipc/bindings';
import { RATES, asWritten, offsetLabel, sameRate } from './useSync';

/**
 * What an alignment says about how it ended.
 *
 * Kept apart from the panel that first showed it, because there are two places
 * to ask from now: the timing panel while a film is playing, and the film sheet
 * before it is. The two are drawn differently and say the same thing, which is
 * only true if the sentences live in one file.
 *
 * Nothing here decides anything. Every ending is a case of the typed outcome
 * Rust returns, and each of them has a wording, including the ones that changed
 * nothing.
 */

/** What the reading is doing at the moment. */
export function working(stage: AlignStageView): string {
  return stage === 'reading' ? 'Listening to the film' : 'Matching the dialogue';
}

/**
 * Where undoing would put the timing back to.
 *
 * A film nobody had adjusted goes back to the file as written rather than back
 * to a value, and the readout says that in words, so the sentence is built
 * around it rather than around a number that is not there.
 */
export function backTo(offsetMs: number): string {
  return offsetMs === 0 ? 'as written' : `to ${offsetLabel(offsetMs)}`;
}

/**
 * What going ahead would throw away, where somebody has already moved the
 * subtitles themselves.
 */
export function replacing(offsetMs: number): string {
  return `The subtitles are moved by ${offsetLabel(offsetMs)} already. Listening to the film will replace that with what it measures.`;
}

/**
 * What to say about how it ended.
 *
 * Each ending that changed nothing says that the bracket keys are still there.
 * Being told that a file cannot be measured is only useful alongside what to do
 * instead.
 */
export function said(outcome: AlignmentView): { title: string; sentence: string } {
  switch (outcome.outcome) {
    case 'aligned':
      return {
        title: 'Lined up',
        sentence: `The subtitles now run ${offsetLabel(outcome.correction.offsetMs)}${rateSaid(
          outcome.correction.rate,
        )}.`,
      };
    case 'too-few-cues':
      return {
        title: 'Not enough to go on',
        sentence: `This subtitle has ${String(outcome.cues)} lines, and it takes about ${String(
          outcome.wanted,
        )} to tell one moment of a film from another. Subtitles for signs and foreign dialogue are usually this short. The bracket keys still work.`,
      };
    case 'no-audio':
      return {
        title: 'Nothing to listen to',
        sentence:
          'This film carries no soundtrack, so there is nothing to measure the subtitles against. The bracket keys still work.',
      };
    case 'unsupported':
      return {
        title: 'Audio Subtext cannot read',
        sentence: `${
          outcome.codec === null ? 'This soundtrack is' : `This soundtrack is ${outcome.codec},`
        } in a format Subtext has no decoder for, so there is nothing to measure against. The bracket keys still work.`,
      };
    case 'uncertain':
      return {
        title: 'Not sure enough to say',
        sentence:
          'The lines in this subtitle do not fall where the talking in this film does, whatever they are moved by. It may belong to a different film or a different cut. Nothing has been changed, and the bracket keys still work.',
      };
    case 'unreadable':
      return { title: 'This film could not be read', sentence: outcome.message };
    case 'stopped':
      return { title: 'Stopped', sentence: 'The subtitles have been left as they were.' };
  }
}

/** The stretch, where there is one, named as the list names it. */
function rateSaid(rate: number): string {
  if (asWritten({ offsetMs: 0, rate })) return '';
  const named = RATES.find((known) => sameRate(known.value, rate));
  return `, stretched ${named?.name ?? `by ${rate.toFixed(4)}`}`;
}
