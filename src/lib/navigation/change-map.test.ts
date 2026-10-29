import { describe, expect, it, vi } from 'vitest';
import { changeMapOpen, mapInvestigationId, restoreMapTrigger, setChangeMap } from './change-map';

describe('Change Map contextual navigation', () => {
  it('preserves origin query parameters and removes only the map view', () => {
    const dispatchEvent = vi.fn();
    const location = { hash: '#/timeline?range=custom&from=10&to=20&filter=network' };
    const history = {
      state: null as unknown,
      pushState: vi.fn((state: unknown, _title: string, value: string) => { history.state = state; location.hash = value; }),
      replaceState: vi.fn((state: unknown, _title: string, value: string) => { history.state = state; location.hash = value; }),
      back: vi.fn(() => { location.hash = '#/timeline?range=custom&from=10&to=20&filter=network'; }),
    };
    const win = {
      location,
      history,
      dispatchEvent,
    } as unknown as Window;

    setChangeMap(true, {}, win);
    expect(win.location.hash).toContain('view=map');
    expect(win.location.hash).toContain('filter=network');
    setChangeMap(false, {}, win);
    expect(win.location.hash).toBe('#/timeline?range=custom&from=10&to=20&filter=network');
    expect(win.history.back).toHaveBeenCalledOnce();
    expect(dispatchEvent).toHaveBeenCalledTimes(1);
  });

  it('removes a directly loaded map view in place instead of adding another history entry', () => {
    const dispatchEvent = vi.fn();
    const win = {
      location: { hash: '#/investigations?id=12&view=map' },
      history: {
        state: null,
        pushState: vi.fn(),
        replaceState: vi.fn((_state, _title, value: string) => { win.location.hash = value; }),
        back: vi.fn(),
      },
      dispatchEvent,
    } as unknown as Window;

    setChangeMap(false, {}, win);
    expect(win.location.hash).toBe('#/investigations?id=12');
    expect(win.history.replaceState).toHaveBeenCalledOnce();
    expect(win.history.back).not.toHaveBeenCalled();
    expect(dispatchEvent).toHaveBeenCalledOnce();
  });

  it('reads an investigation id only when it is a positive safe integer', () => {
    expect(changeMapOpen('#/investigations?id=12&view=map')).toBe(true);
    expect(mapInvestigationId('#/investigations?id=12&view=map')).toBe(12);
    expect(mapInvestigationId('#/investigations?id=NaN&view=map')).toBeNull();
  });

  it('retries focus restoration until the contextual trigger has remounted', () => {
    vi.useFakeTimers();
    let frame: FrameRequestCallback | undefined;
    vi.stubGlobal('requestAnimationFrame', vi.fn((callback: FrameRequestCallback) => { frame = callback; return 1; }));
    const target = document.createElement('button');
    target.dataset.changeMapTrigger = 'timeline';

    restoreMapTrigger('[data-change-map-trigger="timeline"]');
    frame?.(0);
    expect(document.activeElement).not.toBe(target);
    document.body.append(target);
    frame?.(16);
    expect(document.activeElement).toBe(target);
    target.remove();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });
});
