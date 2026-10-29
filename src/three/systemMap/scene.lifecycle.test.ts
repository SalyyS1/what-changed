import { afterEach, describe, expect, it, vi } from 'vitest';
import type { SystemMapDataLike } from './types';

const rendererInstances = vi.hoisted(() => [] as Array<{
  domElement: HTMLCanvasElement;
  animationLoop: ((time: number) => void) | null;
  disposed: boolean;
  contextForced: boolean;
  renderCount: number;
  info: { memory: { geometries: number; textures: number } };
}>);
const pickingCallbacks = vi.hoisted(() => [] as Array<{
  hover(index: number | null, node: unknown, x: number, y: number): void;
  select(index: number, node: unknown): void;
}>);
const pickingDisposals = vi.hoisted(() => [] as ReturnType<typeof vi.fn>[]);

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

vi.mock('../shared/picking', () => ({
  createPickingController: (_element: HTMLElement, _camera: unknown, _mesh: unknown, _nodes: unknown, callbacks: never) => {
    pickingCallbacks.push(callbacks);
    const dispose = vi.fn();
    pickingDisposals.push(dispose);
    return { dispose };
  },
}));

import { createSystemMapScene } from './scene';

const mapData: SystemMapDataLike = {
  nodes: [
    { entity_id: 'process:1', entity_type: 'process', display_name: 'One', hop: 0, degree: 1, last_changed_at: 1, recent_event_count: 1 },
    { entity_id: 'port:2', entity_type: 'port', display_name: 'Two', hop: 1, degree: 1, last_changed_at: 2, recent_event_count: 1 },
  ],
  edges: [{ source: 'process:1', target: 'port:2', kind: 'listens_on', recorded_at: 1, evidence_event_id: 1 }],
  focus: { entity_id: 'process:1', entity_type: 'process' },
  truncated: false,
  total_nodes_available: 2,
};

describe('System Map scene lifecycle and picking', () => {
  afterEach(() => {
    rendererInstances.length = 0;
    pickingCallbacks.length = 0;
    pickingDisposals.length = 0;
    document.body.replaceChildren();
    vi.restoreAllMocks();
  });

  it('disposes renderer resources and direct listeners over 20 cycles', () => {
    const add = vi.spyOn(document, 'addEventListener');
    const remove = vi.spyOn(document, 'removeEventListener');
    for (let cycle = 0; cycle < 20; cycle += 1) {
      const container = document.createElement('div');
      document.body.append(container);
      const scene = createSystemMapScene({ container, data: mapData, reducedMotion: true });
      scene.resize(900, 600);
      scene.destroy();
      expect(container.querySelector('canvas')).toBeNull();
    }
    expect(rendererInstances).toHaveLength(20);
    for (const renderer of rendererInstances) {
      expect(renderer.animationLoop).toBeNull();
      expect(renderer.disposed).toBe(true);
      expect(renderer.contextForced).toBe(true);
      expect(renderer.info.memory).toEqual({ geometries: 0, textures: 0 });
    }
    expect(add.mock.calls.filter(([type]) => type === 'visibilitychange')).toHaveLength(20);
    expect(remove.mock.calls.filter(([type]) => type === 'visibilitychange')).toHaveLength(20);
    expect(pickingDisposals.every((dispose) => dispose.mock.calls.length === 1)).toBe(true);
  });

  it('renders on demand under reduced motion and pauses while inactive or hidden', () => {
    let hidden = false;
    vi.spyOn(document, 'hidden', 'get').mockImplementation(() => hidden);
    const container = document.createElement('div');
    document.body.append(container);
    const reduced = createSystemMapScene({ container, data: mapData, reducedMotion: true });
    const reducedRenderer = rendererInstances[0]!;
    expect(reducedRenderer.animationLoop).toBeNull();
    const count = reducedRenderer.renderCount;
    reduced.setSelected('port:2');
    expect(reducedRenderer.renderCount).toBe(count + 1);
    reduced.destroy();

    const animated = createSystemMapScene({ container, data: mapData, reducedMotion: false });
    const animatedRenderer = rendererInstances[1]!;
    expect(animatedRenderer.animationLoop).toBeTypeOf('function');
    animated.setVisible(false);
    expect(animatedRenderer.animationLoop).toBeNull();
    animated.setVisible(true);
    expect(animatedRenderer.animationLoop).toBeTypeOf('function');
    hidden = true;
    document.dispatchEvent(new Event('visibilitychange'));
    expect(animatedRenderer.animationLoop).toBeNull();
    animated.destroy();
  });

  it('forwards full-node hover and typed focus from shared picking', () => {
    const onHover = vi.fn();
    const onFocus = vi.fn();
    const container = document.createElement('div');
    const scene = createSystemMapScene({ container, data: mapData, reducedMotion: true, onHover, onFocus });
    const callbacks = pickingCallbacks[0]!;
    callbacks.hover(1, mapData.nodes[1], 40, 50);
    expect(onHover).toHaveBeenCalledWith(mapData.nodes[1], 40, 50);
    callbacks.select(1, mapData.nodes[1]);
    expect(onFocus).toHaveBeenCalledWith({ entity_id: 'port:2', entity_type: 'port' });
    scene.destroy();
  });

  it('stops and reports WebGL context loss', () => {
    const onContextLost = vi.fn();
    const container = document.createElement('div');
    const scene = createSystemMapScene({ container, data: mapData, reducedMotion: false, onContextLost });
    const renderer = rendererInstances[0]!;
    renderer.domElement.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
    expect(renderer.animationLoop).toBeNull();
    expect(onContextLost).toHaveBeenCalledOnce();
    scene.destroy();
  });
});
