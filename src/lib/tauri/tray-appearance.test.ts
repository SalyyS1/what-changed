import { describe, expect, it, vi } from 'vitest';
import { watchForcedColors } from './tray-appearance';

function fakeMedia(initial: boolean) {
  let change: (() => void) | undefined;
  const media = {
    matches: initial,
    addEventListener: vi.fn((_type: string, listener: () => void) => { change = listener; }),
    removeEventListener: vi.fn((_type: string, listener: () => void) => {
      if (change === listener) change = undefined;
    }),
  };
  return {
    media,
    emit(active: boolean) {
      media.matches = active;
      change?.();
    },
  };
}

describe('forced-colors tray relay', () => {
  it('reports the initial bit and each change, then detaches', async () => {
    const fake = fakeMedia(false);
    const report = vi.fn(async (_active: boolean) => {});
    const matchMedia = vi.fn(() => fake.media as unknown as MediaQueryList);
    const detach = watchForcedColors(report, matchMedia);

    expect(matchMedia).toHaveBeenCalledWith('(forced-colors: active)');
    expect(report).toHaveBeenCalledWith(false);
    fake.emit(true);
    await Promise.resolve();
    expect(report).toHaveBeenLastCalledWith(true);
    detach();
    fake.emit(false);
    expect(report).toHaveBeenCalledTimes(2);
    expect(fake.media.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function));
  });

  it('serializes rapid IPC changes and applies the latest bit last', async () => {
    const fake = fakeMedia(false);
    const completions: Array<() => void> = [];
    const report = vi.fn((_active: boolean) => new Promise<void>((resolve) => {
      completions.push(resolve);
    }));
    const detach = watchForcedColors(report, () => fake.media as unknown as MediaQueryList);

    expect(report.mock.calls.map(([active]) => active)).toEqual([false]);
    fake.emit(true);
    fake.emit(false);
    fake.emit(true);
    expect(report).toHaveBeenCalledTimes(1);

    completions.shift()?.();
    await Promise.resolve();
    expect(report.mock.calls.map(([active]) => active)).toEqual([false, true]);
    fake.emit(false);
    expect(report).toHaveBeenCalledTimes(2);

    completions.shift()?.();
    await Promise.resolve();
    expect(report.mock.calls.map(([active]) => active)).toEqual([false, true, false]);
    completions.shift()?.();
    await Promise.resolve();
    detach();
  });

  it('is inert when matchMedia is unavailable', () => {
    const report = vi.fn(async (_active: boolean) => {});
    watchForcedColors(report, null)();
    expect(report).not.toHaveBeenCalled();
  });

  it('does not propagate a failed cosmetic update', async () => {
    const fake = fakeMedia(true);
    const report = vi.fn(async (_active: boolean): Promise<void> => {
      throw new Error('tray unavailable');
    });
    watchForcedColors(report, () => fake.media as unknown as MediaQueryList)();
    await Promise.resolve();
    expect(report).toHaveBeenCalledWith(true);
  });
});
