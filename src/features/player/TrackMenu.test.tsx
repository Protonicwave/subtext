import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Id, TrackView } from '@/shared/ipc/bindings';
import { TrackMenu } from './TrackMenu';
import type { TrackChoice } from './useTrack';

function track(id: Id, changes: Partial<TrackView> = {}): TrackView {
  return {
    id,
    path: `/films/Heat.${String(id)}.srt`,
    language: 'en',
    forced: false,
    hearingImpaired: false,
    origin: 'sidecar',
    streamNumber: 0,
    form: 'text',
    matchKind: 'exact',
    cueCount: 1_200,
    correction: { offsetMs: 0, rate: 1 },
    ...changes,
  };
}

function show(tracks: TrackView[], active: TrackView | null, off = false) {
  const choice: TrackChoice = {
    tracks,
    active,
    off,
    available: tracks.length > 0,
    choose: vi.fn(),
  };
  render(<TrackMenu choice={choice} onClose={vi.fn()} />);
  return choice;
}

describe('choosing which subtitle to read', () => {
  it('says what each track is rather than which file it came from', () => {
    const english = track(1);
    const forced = track(2, { forced: true, cueCount: 40 });
    const french = track(3, { language: 'fr', hearingImpaired: true });
    show([english, forced, french], english);

    expect(screen.getByRole('radio', { name: 'English, 1,200 lines' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'English, Forced · 40 lines' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /French/ })).toHaveAccessibleName(
      /For the hard of hearing/,
    );
  });

  it('marks the one being read', () => {
    const english = track(1);
    const french = track(2, { language: 'fr' });
    show([english, french], french);

    expect(screen.getByRole('radio', { name: /French/ })).toBeChecked();
    expect(screen.getByRole('radio', { name: /English/ })).not.toBeChecked();
  });

  it('reads the film with the track that is chosen', async () => {
    const english = track(1);
    const french = track(2, { language: 'fr' });
    const choice = show([english, french], english);

    await userEvent.click(screen.getByRole('radio', { name: /French/ }));

    expect(choice.choose).toHaveBeenCalledWith(french);
  });

  it('offers no subtitles at all as one of the answers', async () => {
    const english = track(1);
    const choice = show([english], english);

    await userEvent.click(screen.getByRole('radio', { name: 'No subtitles' }));

    expect(choice.choose).toHaveBeenCalledWith(null);
  });

  /*
   * Turned off is a state the menu has to show, and it is not the same as the
   * film having no subtitle to show: the tracks are still listed, and going
   * back to one of them is a click.
   */
  it('shows subtitles that have been turned off as the choice in force', () => {
    show([track(1)], null, true);

    expect(screen.getByRole('radio', { name: 'No subtitles' })).toBeChecked();
    expect(screen.getByRole('radio', { name: /English/ })).not.toBeChecked();
  });

  it('closes on Escape without the film taking the key as going back', async () => {
    const closed = vi.fn();
    render(
      <TrackMenu
        choice={{
          tracks: [track(1)],
          active: track(1),
          off: false,
          available: true,
          choose: vi.fn(),
        }}
        onClose={closed}
      />,
    );

    await userEvent.click(screen.getByRole('radio', { name: /English/ }));
    await userEvent.keyboard('{Escape}');

    expect(closed).toHaveBeenCalled();
  });
});
