/**
 * Shell-level UI state that is not data and is not owned by the backend: the
 * sidebar's collapse preference, and the toast queue's front door.
 *
 * Monitoring used to live here as a local flag. It does not any more: the tray
 * can pause monitoring while the window is closed, so the value is
 * authoritative in Rust and `stores/monitoring.svelte.ts` mirrors it. Keeping
 * a second copy here would mean the titlebar pill was wrong every time the
 * tray was used.
 */

import { toasts, type Toast, type ToastTone } from './toast.svelte';
import { TOAST_TTL_MS } from '../design/motion';

export type { Toast, ToastTone };

export class UiStore {
  /** User-pinned collapse; `null` means follow the container query. */
  sidebarCollapsed = $state<boolean | null>(null);

  /** The live toast queue, so components can keep reading `ui.toasts`. */
  get toasts(): Toast[] {
    return toasts.items;
  }

  toast(message: string, tone: ToastTone = 'neutral', ttlMs = TOAST_TTL_MS): void {
    toasts.push(message, tone, ttlMs);
  }

  dismissToast(id: number): void {
    toasts.dismiss(id);
  }
}

export const ui = new UiStore();
