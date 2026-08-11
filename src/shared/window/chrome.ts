import { useEffect, useState } from 'react';
import type { Chrome } from '@/shared/ipc/bindings';
import { ipc } from '@/shared/ipc/client';

/**
 * What the window the front end is drawing into turned out to be.
 *
 * Both halves of the answer are settled when the window is created and cannot
 * change while the application is running, so this asks once and then holds
 * still. A window that will not say what it is answers as the plainest kind of
 * window, which is what every platform but one is anyway.
 */
const PLAIN: Chrome = { backdrop: false, switchableDecoding: false };

export function useChrome(): Chrome {
  const [chrome, setChrome] = useState(PLAIN);

  useEffect(() => {
    let asking = true;
    void ipc
      .windowChrome()
      .then((answer) => {
        if (asking) setChrome(answer);
      })
      .catch(() => {
        // Nothing to tell anybody. The window is drawn the plain way.
      });

    return () => {
      asking = false;
    };
  }, []);

  return chrome;
}
