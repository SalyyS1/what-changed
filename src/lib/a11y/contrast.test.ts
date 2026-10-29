import { describe, expect, it } from 'vitest';
import { contrastRatio } from '../design/contrast';
import { palettes, type ThemeName } from '../design/tokens';
import { contrastPairs, requiredContrastRatio } from './contrast-pairs';

describe('Phase 20 contrast manifest', () => {
  it('covers every declared normal-size pair in both themes', () => {
    expect(contrastPairs.length).toBeGreaterThan(0);
    for (const theme of ['dark', 'light'] as const satisfies readonly ThemeName[]) {
      const palette = palettes[theme];
      for (const pair of contrastPairs) {
        const ratio = contrastRatio(palette[pair.fg], palette[pair.bg]);
        expect(ratio, `${theme}: ${pair.fg} on ${pair.bg}`).toBeGreaterThanOrEqual(
          Math.max(requiredContrastRatio, pair.minRatio),
        );
      }
    }
  });

  it('has no duplicate foreground/background declarations', () => {
    const keys = contrastPairs.map((pair) => `${pair.fg}/${pair.bg}`);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

