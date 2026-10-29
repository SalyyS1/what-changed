import { afterEach, describe, expect, it, vi } from 'vitest';
import { readThreePalette } from '../shared/palette';

describe('Change Map palette', () => {
  afterEach(() => vi.restoreAllMocks());

  it('reads the dedicated category tokens used by both renderers', () => {
    const getPropertyValue = vi.fn((name: string) => name === '--map-category-process' ? '#123456' : '');
    vi.stubGlobal('getComputedStyle', () => ({ getPropertyValue }));
    const palette = readThreePalette({
      process: { token: '--map-category-process', fallbackToken: '--accent' },
    }, document.documentElement);
    expect(palette.process.getHexString()).toBe('123456');
    expect(getPropertyValue).toHaveBeenCalledWith('--map-category-process');
    expect(getPropertyValue).not.toHaveBeenCalledWith('--change-map-process');
  });
});
