import { afterEach, describe, expect, it, vi } from 'vitest';

const rendererInstances = vi.hoisted(() => [] as Array<{
  domElement: HTMLCanvasElement;
  animationLoop: ((time: number) => void) | null;
  disposed: boolean;
  contextForced: boolean;
  renderCount: number;
  info: { memory: { geometries: number; textures: number } };
}>);

vi.mock('three', async (importOriginal) => {
  const actual = await importOriginal<typeof import('three')>();
  class TestRenderer {
    domElement = document.createElement('canvas');
    animationLoop: ((time: number) => void) | null = null;
    disposed = false;
    contextForced = false;
    renderCount = 0;
    info = { memory: { geometries: 1, textures: 0 } };
    renderLists = { dispose: vi.fn() };
    constructor() { rendererInstances.push(this); }
    getContext(): { isContextLost: () => boolean } { return { isContextLost: () => false }; }
    setPixelRatio(): void {}
    setSize(): void {}
    render(): void { this.renderCount += 1; }
    setAnimationLoop(loop: ((time: number) => void) | null): void { this.animationLoop = loop; }
    dispose(): void { this.disposed = true; this.info.memory.geometries = 0; }
    forceContextLoss(): void { this.contextForced = true; }
  }
  return { ...actual, WebGLRenderer: TestRenderer };
});

import { createChangeMapScene } from './createScene';

describe('Change Map scene lifecycle', () => {
  afterEach(() => {
    rendererInstances.length = 0;
    document.body.replaceChildren();
  });

  it('fully tears down the real scene graph and visibility listener over 20 cycles', () => {
    const add = vi.spyOn(document, 'addEventListener');
    const remove = vi.spyOn(document, 'removeEventListener');
    const windowAdd = vi.spyOn(window, 'addEventListener');
    const windowRemove = vi.spyOn(window, 'removeEventListener');
    for (let cycle = 0; cycle < 20; cycle += 1) {
      const container = document.createElement('div');
      document.body.append(container);
      const scene = createChangeMapScene({
        container,
        reducedMotion: true,
        data: {
          nodes: [{
            id: 'event:1', event_id: 1, timestamp_ms: 1, category: 'process', lane: 'process',
            importance: 'important', label: 'Process changed', chain_ids: ['chain:1'], chain_depth: 0,
          }],
          edges: [], symptom_time_ms: 1, window_start_ms: 0, window_end_ms: 2,
        },
      });
      scene.resize(900, 600);
      scene.destroy();
      expect(container.querySelector('canvas')).toBeNull();
    }

    expect(rendererInstances).toHaveLength(20);
    for (const renderer of rendererInstances) {
      expect(renderer.animationLoop).toBeNull();
      expect(renderer.disposed).toBe(true);
      expect(renderer.contextForced).toBe(true);
      expect(renderer.info.memory.geometries).toBe(0);
      expect(renderer.info.memory.textures).toBe(0);
    }
    expect(add.mock.calls.filter(([type]) => type === 'visibilitychange')).toHaveLength(20);
    expect(remove.mock.calls.filter(([type]) => type === 'visibilitychange')).toHaveLength(20);
    expect(windowAdd.mock.calls.filter(([type]) => type === 'scroll')).toHaveLength(20);
    expect(windowRemove.mock.calls.filter(([type]) => type === 'scroll')).toHaveLength(20);
    add.mockRestore();
    remove.mockRestore();
    windowAdd.mockRestore();
    windowRemove.mockRestore();
  });

  it('renders on demand for reduced motion and stays flat while idle', () => {
    const container = document.createElement('div');
    document.body.append(container);
    const scene = createChangeMapScene({ container, data: {
      nodes: [], edges: [], symptom_time_ms: null, window_start_ms: 0, window_end_ms: 1,
    }, reducedMotion: true });
    const renderer = rendererInstances[0]!;
    const afterInitialRender = renderer.renderCount;
    expect(renderer.animationLoop).toBeNull();
    expect(renderer.renderCount).toBe(afterInitialRender);
    scene.setSelected(null);
    expect(renderer.renderCount).toBe(afterInitialRender + 1);
    scene.destroy();
  });

  it('stops while hidden, resumes when visible, and reports a lost context', () => {
    let hidden = false;
    const hiddenSpy = vi.spyOn(document, 'hidden', 'get').mockImplementation(() => hidden);
    const onContextLost = vi.fn();
    const container = document.createElement('div');
    document.body.append(container);
    const scene = createChangeMapScene({ container, data: {
      nodes: [], edges: [], symptom_time_ms: null, window_start_ms: 0, window_end_ms: 1,
    }, reducedMotion: false, onContextLost });
    const renderer = rendererInstances[0]!;
    expect(renderer.animationLoop).toBeTypeOf('function');
    hidden = true;
    document.dispatchEvent(new Event('visibilitychange'));
    expect(renderer.animationLoop).toBeNull();
    hidden = false;
    document.dispatchEvent(new Event('visibilitychange'));
    expect(renderer.animationLoop).toBeTypeOf('function');
    renderer.domElement.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
    expect(renderer.animationLoop).toBeNull();
    expect(onContextLost).toHaveBeenCalledOnce();
    scene.destroy();
    hiddenSpy.mockRestore();
  });

  it('reports animation-frame intervals instead of synchronous render duration', () => {
    const onMetrics = vi.fn();
    const container = document.createElement('div');
    document.body.append(container);
    const scene = createChangeMapScene({ container, data: {
      nodes: [], edges: [], symptom_time_ms: null, window_start_ms: 0, window_end_ms: 1,
    }, reducedMotion: false, onMetrics });
    const renderer = rendererInstances[0]!;
    const frame = renderer.animationLoop!;

    for (let index = 0; index <= 300; index += 1) frame(index * 20);

    expect(renderer.renderCount).toBe(301);
    expect(onMetrics).toHaveBeenCalledOnce();
    expect(onMetrics).toHaveBeenCalledWith(20);
    expect(scene.frameTiming()).toEqual({ samples: 300, median_ms: 20 });
    scene.destroy();
  });
});
