import type { AlignStageView, AlignmentView, LandingView } from '@/shared/ipc/bindings';
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
        )}. ${landed(outcome.landing)} land on the talking, against ${inHundred(
          outcome.asWritten,
        )} before.`,
      };
    case 'no-better':
      return {
        title: 'No better than it is',
        sentence: `${wouldLand(
          outcome.landing,
          outcome.asWritten,
        )} Nothing has been changed, and the bracket keys still work.`,
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
        sentence: `The lines in this subtitle do not fall where the talking in this film does, whatever they are moved by.${measured(
          outcome.asWritten,
        )} It may belong to a different film or a different cut. Nothing has been changed, and the bracket keys still work.`,
      };
    case 'unreadable':
      return { title: 'This film could not be read', sentence: outcome.message };
    case 'stopped':
      return { title: 'Stopped', sentence: 'The subtitles have been left as they were.' };
  }
}

/**
 * How many of a track's lines arrive when somebody starts talking.
 *
 * The one thing an alignment reports that was not read off the correlation that
 * produced it, so it is the only evidence a person has for believing an answer
 * rather than taking it on trust. Quoted in lines per hundred rather than as a
 * percentage, because it is a count of lines and reads as one.
 *
 * It is never quoted alone. No film puts every line on a voice, and a figure on
 * its own invites being read as a mark out of a hundred; beside the same figure
 * for the file as it stands, it says the thing that matters.
 */
function inHundred(landing: LandingView): string {
  return String(Math.round(landing.fraction * 100));
}

function landed(landing: LandingView): string {
  return `${inHundred(landing)} lines in a hundred`;
}

/**
 * What a measurement would do to a file, against what the file already does.
 *
 * The two figures are what the refusal rests on, so they are the sentence rather
 * than a footnote to it. Where nothing could be measured there is no comparison
 * to draw and none is invented: none out of none is not a figure.
 */
function wouldLand(landing: LandingView, asWritten: LandingView): string {
  if (landing.examined === 0) {
    return 'There was nothing in this film to measure the subtitle against.';
  }
  return `Measured and applied, ${landed(landing)} would land on the talking, against ${inHundred(
    asWritten,
  )} as the file stands.`;
}

/**
 * What the file manages as it stands, where there was anything to measure.
 *
 * A film with no speech found in it and a track whose lines all fall past the
 * end of the film both come back having measured nothing, and a figure of none
 * out of none would read as a track that lands nowhere rather than as a question
 * nobody could answer.
 */
function measured(asWritten: LandingView): string {
  if (asWritten.examined === 0) return '';
  return ` As the file stands, ${landed(asWritten)} land on the talking.`;
}

/** The stretch, where there is one, named as the list names it. */
function rateSaid(rate: number): string {
  if (asWritten({ offsetMs: 0, rate })) return '';
  const named = RATES.find((known) => sameRate(known.value, rate));
  return `, stretched ${named?.name ?? `by ${rate.toFixed(4)}`}`;
}
