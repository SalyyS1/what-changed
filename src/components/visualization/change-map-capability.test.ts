import { describe, expect, it, vi } from 'vitest';
import { probeWebGl } from './change-map-capability';

describe('Change Map capability probe', () => {
  it('explicitly releases the throwaway WebGL context', () => {
    const loseContext = vi.fn();
    const getContext = vi.fn().mockReturnValue({
      getExtension: vi.fn((name: string) => name === 'WEBGL_lose_context' ? { loseContext } : null),
    });

    expect(probeWebGl({ getContext } as unknown as HTMLCanvasElement, 'auto')).toBe(true);
    expect(loseContext).toHaveBeenCalledOnce();
  });

  it('does not allocate a probe context when 2D is forced', () => {
    const getContext = vi.fn();
    expect(probeWebGl({ getContext } as unknown as HTMLCanvasElement, '2d')).toBe(true);
    expect(getContext).not.toHaveBeenCalled();
  });
});
