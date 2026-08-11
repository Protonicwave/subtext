import { create } from 'zustand';

/**
 * Whether the transcript is beside the film.
 *
 * Held here rather than in the player, so that it is answered the same way for
 * the next film as for this one. Somebody who put the panel away is not asking
 * to be shown it again by opening something else.
 *
 * It starts open. The transcript is the reason this application exists, and a
 * feature nobody finds is a feature nobody has.
 */
interface Panel {
  readonly open: boolean;
  readonly toggle: () => void;
  readonly close: () => void;
}

export const usePanel = create<Panel>((set) => ({
  open: true,
  toggle: () => {
    set((panel) => ({ open: !panel.open }));
  },
  close: () => {
    set({ open: false });
  },
}));
