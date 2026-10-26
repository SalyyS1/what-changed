/**
 * Settings, loaded once and written through.
 *
 * The store owns an `AsyncState` so the Settings screen renders the same four
 * faces every other screen does. A save is optimistic in the sense that the
 * backend's *answer* replaces the local copy — it returns the settings as they
 * now stand, so the screen shows what was stored rather than what was sent,
 * and a value the backend clamped is visible immediately.
 */

import { theme } from '../design/theme.svelte';
import { failed, loading, ready, type AsyncState } from '../types/async-state';
import type { Settings, SettingsUpdate } from '../tauri/bindings';
import { getSettings, toAppError, updateSettings } from '../tauri/client';

export class SettingsStore {
  state = $state<AsyncState<Settings>>(loading());
  /** True while a save is in flight, so a toggle can disable itself. */
  saving = $state(false);

  /** The settings if they are loaded, otherwise `null`. */
  get current(): Settings | null {
    return this.state.status === 'ready' ? this.state.data : null;
  }

  async load(): Promise<void> {
    // A refresh keeps the loaded settings on screen. Blanking `current` mid-load
    // made App unmount `OnboardingFlow`, which reloads settings when it mounts,
    // so a profile with onboarding unfinished re-mounted every screen in a
    // tight loop (about 63 round-trips a second, three cores of CPU).
    if (this.state.status !== 'ready') this.state = loading();
    try {
      const settings = await getSettings();
      this.state = ready(settings);
      this.#hydrate(settings);
    } catch (reason) {
      const error = toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => void this.load() : undefined);
    }
  }

  /**
   * Apply the stored settings to the stores that own them locally.
   *
   * The theme is the one setting with two homes: `localStorage`, which
   * `public/theme-bootstrap.js` reads synchronously before first paint, and
   * the database, which is where it belongs. Rust is the source of truth, so
   * loading has to *push* the stored value into the theme store — without
   * this the mirror is the only reader and the database column is write-only,
   * which is what the round-trip was quietly doing before.
   *
   * Writing it back through `theme.set` also refreshes the mirror, so the
   * next cold start paints the stored theme rather than the last one this
   * machine happened to choose.
   */
  #hydrate(settings: Settings): void {
    if (theme.mode !== settings.theme) {
      theme.set(settings.theme);
    }
  }

  /**
   * Apply a partial change. Returns whether it was stored, so a caller that
   * needs to revert a control knows without reading the state back.
   */
  async save(update: SettingsUpdate): Promise<boolean> {
    this.saving = true;
    try {
      const settings = await updateSettings(update);
      this.state = ready(settings);
      // The backend returns the settings as they now stand, so a value it
      // clamped or refused is reflected here rather than in the control.
      this.#hydrate(settings);
      return true;
    } catch (reason) {
      const error = toAppError(reason);
      // A failed save must not blank the screen: the previous settings are
      // still what is stored, and they are still what should be shown.
      if (this.state.status !== 'ready') {
        this.state = failed(error.message);
      }
      return false;
    } finally {
      this.saving = false;
    }
  }
}

export const settings = new SettingsStore();
