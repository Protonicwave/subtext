import { useEffect, useRef } from 'react';

/** The shape the generated bindings give every event. */
interface Emitter<Payload> {
  listen: (handler: (event: { payload: Payload }) => void) => Promise<() => void>;
}

/**
 * Listens to a back end event for as long as a component is mounted.
 *
 * Subscribing is asynchronous and unmounting is not, so a component that
 * mounts and unmounts quickly can be handed its unsubscribe function after it
 * has already gone. Every listener in the application would otherwise repeat
 * the same few lines to deal with that, so it lives here instead.
 *
 * The handler is read through a ref rather than depended on, so that passing an
 * inline function does not tear the subscription down and set it up again on
 * every render.
 */
export function useIpcEvent<Payload>(
  emitter: Emitter<Payload>,
  handler: (payload: Payload) => void,
): void {
  const latest = useRef(handler);
  useEffect(() => {
    latest.current = handler;
  }, [handler]);

  useEffect(() => {
    let listening = true;
    let stop: (() => void) | undefined;

    void emitter
      .listen((event) => {
        latest.current(event.payload);
      })
      .then((unsubscribe) => {
        if (listening) stop = unsubscribe;
        // Already unmounted by the time the subscription was ready.
        else unsubscribe();
      });

    return () => {
      listening = false;
      stop?.();
    };
  }, [emitter]);
}
