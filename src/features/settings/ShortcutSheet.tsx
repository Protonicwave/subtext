import { useEffect, useState } from 'react';
import { Sheet } from '@/shared/ui/Sheet';
import { ShortcutTable } from './ShortcutTable';

/**
 * The keys, brought up by the question mark from wherever somebody is.
 *
 * A shortcut nobody has been told about is a shortcut nobody has, and the
 * settings screen is the wrong place to have to go to be told: the moment the
 * question comes up is the moment a film is playing.
 *
 * Nothing here holds a preference, so the state is the component's own rather
 * than a store's.
 */
export function ShortcutSheet() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
      // A question mark typed into a field is a question mark.
      if (event.key !== '?' || isTyping(event.target)) return;

      setOpen((showing) => !showing);
      event.preventDefault();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  return (
    <Sheet
      open={open}
      title="Keys"
      description="Press the question mark from anywhere to bring this back."
      onClose={() => {
        setOpen(false);
      }}
    >
      <ShortcutTable />
    </Sheet>
  );
}

/** Whether the key belongs to a control that reads keys itself. */
function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;

  return (
    target.isContentEditable ||
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement
  );
}
