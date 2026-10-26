/**
 * The one shape every screen's data takes. `StateShell` renders exactly one
 * branch of it, which is how the six-state rule (§70) becomes a type instead
 * of a review comment: a screen cannot forget its empty or error state
 * because it cannot construct a value without choosing one.
 */

export type AsyncState<T> =
  | { status: 'loading' }
  | { status: 'empty'; reason?: string | undefined }
  | { status: 'error'; message: string; retry?: (() => void) | undefined }
  | { status: 'ready'; data: T };

export const loading = <T>(): AsyncState<T> => ({ status: 'loading' });
export const empty = <T>(reason?: string): AsyncState<T> => ({ status: 'empty', reason });
export const failed = <T>(message: string, retry?: () => void): AsyncState<T> => ({
  status: 'error',
  message,
  retry,
});
export const ready = <T>(data: T): AsyncState<T> => ({ status: 'ready', data });

export function isReady<T>(s: AsyncState<T>): s is { status: 'ready'; data: T } {
  return s.status === 'ready';
}
