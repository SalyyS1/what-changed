/**
 * Theme state. The mode (`dark` | `light` | `system`) is what the user chose;
 * the resolved theme is what is painted. `system` tracks the OS through
 * `matchMedia` for as long as the app runs.
 *
 * Persistence: the mode lives in a single `localStorage` key. Once Rust
 * settings own it, the key remains as a synchronous mirror that
 * `public/theme-bootstrap.js` reads before first paint — which is what
 * prevents a flash of the wrong theme on cold start.
 */

import type { ThemeName } from './tokens';

export type ThemeMode = ThemeName | 'system';

export const THEME_STORAGE_KEY = 'wc.theme';
/** Cycle order for a single toggle control. */
export const THEME_MODES: readonly ThemeMode[] = ['dark', 'light', 'system'];
const MODES = THEME_MODES;
const QUERY = '(prefers-color-scheme: dark)';

function isThemeMode(v: unknown): v is ThemeMode {
  return typeof v === 'string' && (MODES as readonly string[]).includes(v);
}

function readStored(): ThemeMode {
  try {
    const v = globalThis.localStorage?.getItem(THEME_STORAGE_KEY);
    return isThemeMode(v) ? v : 'dark';
  } catch {
    return 'dark';
  }
}

function systemPrefersDark(): boolean {
  return globalThis.matchMedia?.(QUERY).matches ?? true;
}

export class ThemeStore {
  mode = $state<ThemeMode>(readStored());
  #systemDark = $state(systemPrefersDark());
  #unsubscribe: (() => void) | null = null;

  /** The theme actually applied to <html>. */
  readonly resolved: ThemeName = $derived(
    this.mode === 'system' ? (this.#systemDark ? 'dark' : 'light') : this.mode,
  );

  /** Bind once from the root component; returns the teardown. */
  attach(doc: Document = document): () => void {
    const mql = globalThis.matchMedia?.(QUERY);
    const onChange = (e: MediaQueryListEvent) => {
      this.#systemDark = e.matches;
    };
    mql?.addEventListener('change', onChange);

    const stop = $effect.root(() => {
      $effect(() => {
        doc.documentElement.dataset.theme = this.resolved;
      });
      $effect(() => {
        try {
          globalThis.localStorage?.setItem(THEME_STORAGE_KEY, this.mode);
        } catch {
          // Storage can be unavailable in a locked-down profile; the theme
          // still applies for this session, it just will not survive restart.
        }
      });
    });

    this.#unsubscribe = () => {
      mql?.removeEventListener('change', onChange);
      stop();
    };
    return this.#unsubscribe;
  }

  set(mode: ThemeMode): void {
    this.mode = mode;
  }

  /** Dark → Light → System → Dark, for a single toggle control. */
  cycle(): void {
    const i = MODES.indexOf(this.mode);
    this.mode = MODES[(i + 1) % MODES.length] ?? 'dark';
  }
}

export const theme = new ThemeStore();
