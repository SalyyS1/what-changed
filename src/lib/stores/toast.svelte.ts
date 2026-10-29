/**
 * The transient message queue.
 *
 * Moved out of `ui.svelte.ts` when the backend gained a voice: a toast is now
 * raised from a command failure as often as from a user action, and a store
 * that also owns the sidebar's collapse preference is not a store a client
 * error handler should be importing.
 *
 * `ui.toast()` still forwards here, so no screen had to change.
 */

import { TOAST_TTL_MS } from '../design/motion';

export type ToastTone = 'neutral' | 'success' | 'warning' | 'danger';

export interface Toast {
  id: number;
  message: string;
  tone: ToastTone;
}

/** Most toasts on screen at once. Beyond this the oldest is dropped: a stack
 * taller than the window is a stack nobody reads. */
const MAX_VISIBLE = 4;

export class ToastStore {
  items = $state<Toast[]>([]);
  #next = 1;

  push(message: string, tone: ToastTone = 'neutral', ttlMs = TOAST_TTL_MS): number {
    const id = this.#next++;
    const queued = [...this.items, { id, message, tone }];
    this.items = queued.slice(-MAX_VISIBLE);
    if (ttlMs > 0) {
      setTimeout(() => this.dismiss(id), ttlMs);
    }
    return id;
  }

  /** Report a backend failure. Retryable failures read as a warning rather
   * than a danger, because the user's next action is to try again. */
  fromError(error: { message: string; retryable: boolean }): number {
    return this.push(error.message, error.retryable ? 'warning' : 'danger');
  }

  dismiss(id: number): void {
    this.items = this.items.filter((t) => t.id !== id);
  }

  clear(): void {
    this.items = [];
  }
}

export const toasts = new ToastStore();
