import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { AlignmentView } from '@/shared/ipc/bindings';
import { landing, nothingMeasured } from '@/test/alignment';
import { SyncPanel } from './SyncPanel';
import type { Alignment, AlignState } from './useAlignment';
import { RATES, STEP_MS, type Sync } from './useSync';

function show(state: Partial<Sync> = {}, phase: AlignState = { phase: 'idle' }) {
  const sync: Sync = {
    available: true,
    offsetMs: 0,
    rate: 1,
    pending: false,
    nudge: vi.fn(),
    setRate: vi.fn(),
    reset: vi.fn(),
    apply: vi.fn(() => Promise.resolve()),
    ...state,
  };
  const alignment: Alignment = {
    available: true,
    state: phase,
    start: vi.fn(),
    confirm: vi.fn(),
    cancel: vi.fn(),
    undo: vi.fn(),
    dismiss: vi.fn(),
  };
  const onClose = vi.fn();

  render(<SyncPanel sync={sync} alignment={alignment} onClose={onClose} />);
  return { sync, alignment, onClose };
}

/** The panel showing how an alignment ended. */
function ended(outcome: AlignmentView, sync: Partial<Sync> = {}) {
  return show(sync, { phase: 'outcome', outcome, undone: false });
}

describe('the subtitle timing controls', () => {
  it('says which way the subtitles have been moved and by how much', () => {
    show({ offsetMs: -1_250 });
    expect(screen.getByText('−1.25s')).toBeInTheDocument();
  });

  it('says that a file nobody has touched is as it was written', () => {
    show();
    expect(screen.getByText('as written')).toBeInTheDocument();
  });

  it('moves the subtitles a step at a time, either way', async () => {
    const { sync } = show();

    await userEvent.click(screen.getByRole('button', { name: 'Later' }));
    expect(sync.nudge).toHaveBeenCalledWith(STEP_MS);

    await userEvent.click(screen.getByRole('button', { name: 'Earlier' }));
    expect(sync.nudge).toHaveBeenCalledWith(-STEP_MS);
  });

  it('offers nothing to undo on a file nobody has touched', () => {
    show();
    expect(screen.getByRole('button', { name: 'As written' })).toBeDisabled();
  });

  it('goes back to the file as it was written', async () => {
    const { sync } = show({ offsetMs: 400 });

    await userEvent.click(screen.getByRole('button', { name: 'As written' }));
    expect(sync.reset).toHaveBeenCalled();
  });

  it('offers the conversions that actually occur rather than a slider', async () => {
    const { sync } = show();
    const list = screen.getByRole('combobox', { name: /framerate/i });

    // A ratio between two framerates. Anybody who needs another can type it,
    // and nobody has to find one by dragging.
    const wanted = RATES[1];
    await userEvent.selectOptions(list, String(wanted?.value));
    expect(sync.setRate).toHaveBeenCalledWith(wanted?.value);

    expect(screen.getByRole('option', { name: 'A value of my own' })).toBeInTheDocument();
  });

  it('shows the rate a film is already on', () => {
    show({ rate: RATES[2]?.value ?? 1 });
    expect(screen.getByRole('combobox', { name: /framerate/i })).toHaveValue(
      String(RATES[2]?.value),
    );
  });

  it('takes a value of its own once one is asked for', async () => {
    const { sync } = show();

    await userEvent.selectOptions(screen.getByRole('combobox', { name: /framerate/i }), 'other');
    const field = screen.getByRole('spinbutton', { name: /rate/i });
    await userEvent.clear(field);
    await userEvent.type(field, '1.05');
    await userEvent.tab();

    expect(sync.setRate).toHaveBeenCalledWith(1.05);
  });

  it('closes on Escape rather than leaving the film', async () => {
    const { onClose } = show();

    await userEvent.click(screen.getByRole('button', { name: 'Later' }));
    await userEvent.keyboard('{Escape}');

    expect(onClose).toHaveBeenCalled();
  });
});

describe('lining a subtitle up by listening to the film', () => {
  it('offers to measure a film nobody has adjusted', async () => {
    const { alignment } = show();

    await userEvent.click(screen.getByRole('button', { name: /listen and line up/i }));
    expect(alignment.start).toHaveBeenCalled();
  });

  it('says how far it has got, and can be stopped', async () => {
    const { alignment } = show({}, { phase: 'running', stage: 'reading', fraction: 0.42 });

    expect(screen.getByText('Listening to the film')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '42');

    await userEvent.click(screen.getByRole('button', { name: 'Stop' }));
    expect(alignment.cancel).toHaveBeenCalled();
  });

  it('names the second half of the work as something other than the first', () => {
    show({}, { phase: 'running', stage: 'correlating', fraction: 1 });
    expect(screen.getByText('Matching the dialogue')).toBeInTheDocument();
  });

  /*
   * A value somebody arrived at themselves is evidence about the file, so it is
   * not thrown away without being asked about. Nothing records where a
   * correction came from, so anything other than the file as written counts.
   */
  it('asks before replacing a correction that is already in force', async () => {
    const { alignment } = show({ offsetMs: -1_250 }, { phase: 'confirming' });

    expect(screen.getByText(/moved by −1.25s already/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Keep mine' }));
    expect(alignment.dismiss).toHaveBeenCalled();
    expect(alignment.confirm).not.toHaveBeenCalled();
  });

  it('goes ahead once that is answered', async () => {
    const { alignment } = show({ offsetMs: -1_250 }, { phase: 'confirming' });

    await userEvent.click(screen.getByRole('button', { name: 'Measure anyway' }));
    expect(alignment.confirm).toHaveBeenCalled();
  });

  it('says what it found in the terms the readout uses', () => {
    ended({
      outcome: 'aligned',
      correction: { offsetMs: 3_500, rate: 1 },
      previous: { offsetMs: 0, rate: 1 },
      confidence: 0.6,
      landing: landing(0.94),
      asWritten: landing(0.14),
      reference: { against: 'speech' },
    });

    expect(screen.getByText('Lined up')).toBeInTheDocument();
    expect(screen.getByText(/now run \+3.50s/)).toBeInTheDocument();
  });

  /*
   * The one thing an alignment reports that did not come out of the correlation
   * that produced it, and so the only reason anybody has to believe the answer
   * rather than take it on trust. It is quoted against the file as it stands,
   * since a share of lines landing means nothing without something to compare
   * it with.
   */
  it('says how many lines land now, against how many landed before', () => {
    ended({
      outcome: 'aligned',
      correction: { offsetMs: 3_500, rate: 1 },
      previous: { offsetMs: 0, rate: 1 },
      confidence: 0.6,
      landing: landing(0.94),
      asWritten: landing(0.14),
      reference: { against: 'speech' },
    });

    expect(screen.getByRole('status')).toHaveTextContent(
      /94 lines in a hundred land on the talking, against 14 before/,
    );
  });

  /*
   * A film that carries its own text subtitle track is measured against those
   * timings instead of against its audio, which is a different kind of answer:
   * authored timings on both sides, accurate to the bin, and arrived at without
   * a sample being decoded. The sentence names the reference, because somebody
   * reading it should be able to tell which of the two they have been handed,
   * and it does not call another subtitle track a sound.
   */
  it('names the subtitle track it measured against, where it used one', () => {
    ended({
      outcome: 'aligned',
      correction: { offsetMs: 3_500, rate: 1 },
      previous: { offsetMs: 0, rate: 1 },
      confidence: 0.6,
      landing: landing(0.94),
      asWritten: landing(0.14),
      reference: { against: 'track', language: 'en', inside: true },
    });

    expect(screen.getByRole('status')).toHaveTextContent(
      /measured against the English subtitles inside the film/,
    );
    expect(screen.getByRole('status')).toHaveTextContent(
      /94 lines in a hundred line up with it, against 14 before/,
    );
  });

  /*
   * The other side of the same sentence. A reference paired to the film by name
   * is a weaker claim than one muxed into it, and a track whose file says
   * nothing about its language cannot be given one.
   */
  it('says where a reference came from, and does not invent a language for it', () => {
    ended({
      outcome: 'aligned',
      correction: { offsetMs: -900, rate: 1 },
      previous: { offsetMs: 0, rate: 1 },
      confidence: 0.5,
      landing: landing(0.91),
      asWritten: landing(0.2),
      reference: { against: 'track', language: null, inside: false },
    });

    expect(screen.getByRole('status')).toHaveTextContent(
      /measured against the subtitles beside the film/,
    );
  });

  /*
   * A correlation can be perfectly clear about a lag and still be answering the
   * wrong question, since the confidence beside it is read off the same peak.
   * This is the ending where the answer was put to the film and did not help,
   * and nothing was written.
   */
  it('says when a measurement it believed would not have helped', () => {
    ended({
      outcome: 'no-better',
      correction: { offsetMs: -2_400, rate: 1 },
      landing: landing(0.21),
      asWritten: landing(0.33),
      wanted: 0.4,
    });

    expect(screen.getByText('No better than it is')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent(
      /21 lines in a hundred would land on the talking, against 33 as the file stands/,
    );
    expect(screen.getByRole('status')).toHaveTextContent(/Nothing has been changed/);
    expect(screen.queryByRole('button', { name: /put it back/i })).not.toBeInTheDocument();
  });

  it('names a stretch as the list of framerates names it', () => {
    ended({
      outcome: 'aligned',
      correction: { offsetMs: 0, rate: RATES[1]?.value ?? 1 },
      previous: { offsetMs: 0, rate: 1 },
      confidence: 0.6,
      landing: landing(0.94),
      asWritten: landing(0.14),
      reference: { against: 'speech' },
    });

    expect(screen.getByText(/stretched 23.976 to 25 fps/)).toBeInTheDocument();
  });

  it('offers to put back what it replaced, in one press', async () => {
    const { alignment } = ended({
      outcome: 'aligned',
      correction: { offsetMs: 3_500, rate: 1 },
      previous: { offsetMs: -800, rate: 1 },
      confidence: 0.6,
      landing: landing(0.94),
      asWritten: landing(0.14),
      reference: { against: 'speech' },
    });

    await userEvent.click(screen.getByRole('button', { name: /put it back to −0.80s/i }));
    expect(alignment.undo).toHaveBeenCalled();
  });

  it('has nothing to put back once it has been put back', () => {
    show(
      {},
      {
        phase: 'outcome',
        undone: true,
        outcome: {
          outcome: 'aligned',
          correction: { offsetMs: 3_500, rate: 1 },
          previous: { offsetMs: -800, rate: 1 },
          confidence: 0.6,
          landing: landing(0.94),
          asWritten: landing(0.14),
          reference: { against: 'speech' },
        },
      },
    );

    expect(screen.queryByRole('button', { name: /put it back/i })).not.toBeInTheDocument();
  });

  it('says a film nobody had touched goes back to the file as written', () => {
    ended({
      outcome: 'aligned',
      correction: { offsetMs: 3_500, rate: 1 },
      previous: { offsetMs: 0, rate: 1 },
      confidence: 0.6,
      landing: landing(0.94),
      asWritten: landing(0.14),
      reference: { against: 'speech' },
    });

    expect(screen.getByRole('button', { name: 'Put it back as written' })).toBeInTheDocument();
  });

  /*
   * Every ending the back end can report has a wording, and the ones that
   * changed nothing say what to do instead rather than only what went wrong.
   */
  it('says a track with too few lines is too short to go on', () => {
    ended({ outcome: 'too-few-cues', cues: 42, wanted: 100 });

    expect(screen.getByText('Not enough to go on')).toBeInTheDocument();
    expect(screen.getByText(/42 lines/)).toBeInTheDocument();
    expect(screen.getByText(/bracket keys still work/)).toBeInTheDocument();
  });

  it('says a film with no soundtrack has nothing to listen to', () => {
    ended({ outcome: 'no-audio' });
    expect(screen.getByText('Nothing to listen to')).toBeInTheDocument();
  });

  it('names the audio it cannot read, in the register the codec note uses', () => {
    ended({ outcome: 'unsupported', codec: 'AC-3' });

    expect(screen.getByText(/This soundtrack is AC-3/)).toBeInTheDocument();
    expect(screen.getByText(/no decoder for/)).toBeInTheDocument();
  });

  it('still says something about audio it cannot even name', () => {
    ended({ outcome: 'unsupported', codec: null });
    expect(screen.getByText(/This soundtrack is in a format/)).toBeInTheDocument();
  });

  it('says a measurement it does not believe changed nothing', () => {
    ended({
      outcome: 'uncertain',
      correction: { offsetMs: 12_000, rate: 1 },
      confidence: 0.01,
      wanted: 0.25,
      landing: landing(0.09),
      asWritten: landing(0.08),
    });

    expect(screen.getByText('Not sure enough to say')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent(
      /As the file stands, 8 lines in a hundred land on the talking/,
    );
    expect(screen.getByRole('status')).toHaveTextContent(/Nothing has been changed/);
    // The value it would have written is not offered as a number to try, since
    // a measurement nobody believes is not a suggestion.
    expect(screen.queryByRole('button', { name: /put it back/i })).not.toBeInTheDocument();
  });

  /*
   * A film with no speech found in it, and a subtitle whose lines all fall past
   * the end of it, both measure nothing at all. None out of none is not a figure
   * and reads as a track landing nowhere, so it is left unsaid.
   */
  it('quotes no figure where there was nothing to measure', () => {
    ended({
      outcome: 'uncertain',
      correction: { offsetMs: 0, rate: 1 },
      confidence: 0,
      wanted: 0.25,
      landing: nothingMeasured,
      asWritten: nothingMeasured,
    });

    expect(screen.getByRole('status')).not.toHaveTextContent(/lines in a hundred/);
  });

  it('passes on what could not be read', () => {
    ended({ outcome: 'unreadable', message: 'the file ends part way through a cluster' });
    expect(screen.getByText(/ends part way through a cluster/)).toBeInTheDocument();
  });

  it('says that stopping left the subtitles alone', () => {
    ended({ outcome: 'stopped' });
    expect(screen.getByText(/left as they were/)).toBeInTheDocument();
  });

  it('puts an outcome away when it is dismissed', async () => {
    const { alignment } = ended({ outcome: 'no-audio' });

    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(alignment.dismiss).toHaveBeenCalled();
  });

  it('leaves the nudge buttons working throughout', async () => {
    const { sync } = show({}, { phase: 'running', stage: 'reading', fraction: 0.1 });

    await userEvent.click(screen.getByRole('button', { name: 'Later' }));
    expect(sync.nudge).toHaveBeenCalledWith(STEP_MS);
  });
});
