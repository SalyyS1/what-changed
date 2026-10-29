import { describe, expect, it } from 'vitest';
import { resolveChangeMapRenderer } from './change-map-renderer';

describe('resolveChangeMapRenderer', () => {
  it('uses 3D for the automatic mode when motion and WebGL are available', () => {
    expect(resolveChangeMapRenderer({
      preference: 'auto', reducedMotion: false, webglAvailable: true, contextLost: false,
    })).toBe('3d');
  });

  it('uses 2D automatically for reduced-motion users', () => {
    expect(resolveChangeMapRenderer({
      preference: 'auto', reducedMotion: true, webglAvailable: true, contextLost: false,
    })).toBe('2d');
  });

  it('honours an explicit 3D choice while disabling animation separately', () => {
    expect(resolveChangeMapRenderer({
      preference: '3d', reducedMotion: true, webglAvailable: true, contextLost: false,
    })).toBe('3d');
  });

  it.each([
    { webglAvailable: false, contextLost: false },
    { webglAvailable: true, contextLost: true },
  ])('falls back after an unusable WebGL context: %o', ({ webglAvailable, contextLost }) => {
    expect(resolveChangeMapRenderer({
      preference: '3d', reducedMotion: false, webglAvailable, contextLost,
    })).toBe('2d');
  });
});
