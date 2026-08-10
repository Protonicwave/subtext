import { commands, type Failure } from './bindings';

/**
 * The commands, with their results unwrapped.
 *
 * The generated bindings hand back a result object rather than throwing, which
 * is the honest shape for a boundary that can fail. It is not the shape a
 * screen wants at every call site, where the answer is either the value or
 * something to show the person, so this turns one into the other once.
 *
 * Wrapped generically rather than command by command, so that a command added
 * in Rust appears here the moment the bindings are regenerated and cannot be
 * forgotten.
 */

/** What every generated command resolves to. */
type Answer<T> = { status: 'ok'; data: T } | { status: 'error'; error: Failure };

type Unwrapped<Command> = Command extends (...args: infer Args) => Promise<Answer<infer Value>>
  ? (...args: Args) => Promise<Value>
  : never;

type Client = { [Name in keyof typeof commands]: Unwrapped<(typeof commands)[Name]> };

/**
 * A command that failed, carrying the sentence Rust wrote for it.
 *
 * A distinct type so that a screen can tell the difference between the back end
 * refusing something, which is worth showing, and the front end having a bug,
 * which is not something to put in front of a person.
 */
export class IpcError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'IpcError';
  }
}

export function unwrap<T>(answer: Answer<T>): T {
  if (answer.status === 'error') throw new IpcError(answer.error.message);
  return answer.data;
}

/** The message to show for something that failed, whatever it turned out to be. */
export function reasonFor(failure: unknown): string {
  if (failure instanceof IpcError) return failure.message;
  if (failure instanceof Error) return failure.message;
  return String(failure);
}

/*
 * The cast is the one place the generic wrapping has to be asserted rather than
 * inferred: the mapped type above says what the result is, and building it by
 * iterating the object cannot prove it. Everything either side of this line is
 * checked, and a command whose shape does not fit Unwrapped resolves to never,
 * which fails at its call sites rather than here.
 */
export const ipc = Object.fromEntries(
  Object.entries(commands).map(([name, command]) => [
    name,
    async (...args: unknown[]) =>
      unwrap(await (command as (...args: unknown[]) => Promise<Answer<unknown>>)(...args)),
  ]),
) as Client;
