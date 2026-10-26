import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { THEME_MODES, THEME_STORAGE_KEY } from './theme.svelte';

/**
 * The flash-free cold start depends on `public/theme-bootstrap.js` and the
 * ThemeStore agreeing on the storage key and the mode literals. The script
 * is plain JS outside the bundle, so it cannot import the constants; this
 * pins the duplication instead so a rename in one place fails here.
 */
const script = readFileSync(
  join(import.meta.dirname, '..', '..', '..', 'public', 'theme-bootstrap.js'),
  'utf8',
);

describe('theme bootstrap script parity', () => {
  it('reads the same storage key as the store', () => {
    expect(script).toContain(`localStorage.getItem('${THEME_STORAGE_KEY}')`);
  });

  it('accepts exactly the modes the store accepts', () => {
    for (const mode of THEME_MODES) {
      expect(script, `mode "${mode}" recognised`).toContain(`stored === '${mode}'`);
    }
    const accepted = [...script.matchAll(/stored === '([a-z]+)'/g)].map((m) => m[1]);
    expect(accepted).toHaveLength(THEME_MODES.length);
  });

  it('applies the result through the attribute the stylesheet selects on', () => {
    expect(script).toContain("setAttribute('data-theme'");
  });
});
