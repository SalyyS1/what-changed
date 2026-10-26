/**
 * The custom titlebar's three buttons, behind one small seam. `getCurrentWindow`
 * dereferences `window.__TAURI_INTERNALS__` on call, which does not exist in
 * Vitest's jsdom or in a plain browser tab, so every entry point resolves the
 * window lazily and degrades to a no-op instead of throwing during render.
 *
 * `close` requests a close rather than destroying the window. The backend
 * intercepts that request and hides to the tray instead, so the button reads
 * as "close" and behaves as "put away, still recording" — with one exception
 * the backend owns: if the tray icon could not be created, the close is
 * allowed through, because hiding into a tray that is not there would leave a
 * process the user cannot reach.
 */

import { getCurrentWindow, type Window } from '@tauri-apps/api/window';

function current(): Window | null {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

export const windowControls = {
  available(): boolean {
    return current() !== null;
  },

  async minimize(): Promise<void> {
    await current()?.minimize();
  },

  async toggleMaximize(): Promise<void> {
    await current()?.toggleMaximize();
  },

  /** Asks to close. The backend turns this into a hide when a tray exists. */
  async close(): Promise<void> {
    await current()?.close();
  },

  async isMaximized(): Promise<boolean> {
    return (await current()?.isMaximized()) ?? false;
  },

  /**
   * Fires with the new maximized state whenever the window is resized. Returns
   * the unlisten function, or a no-op outside Tauri.
   */
  async onMaximizedChange(cb: (maximized: boolean) => void): Promise<() => void> {
    const win = current();
    if (!win) {
      return () => {};
    }
    return win.onResized(async () => cb(await win.isMaximized()));
  },
};
