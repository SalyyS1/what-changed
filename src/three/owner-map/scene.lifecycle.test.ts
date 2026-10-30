import { afterEach, describe, expect, it, vi } from 'vitest';
import { BufferGeometry, Material } from 'three';
import { ownerGraphFixture } from '../../lib/owner-graph/test-fixtures';
import { projectOwnerGraph } from '../../lib/owner-graph/projection';
import { ownerGraphWindow } from '../../lib/owner-graph/selection';

const instances = vi.hoisted(() => [] as Array<{ domElement: HTMLCanvasElement; renders: number; disposed: boolean; lost: boolean; animationLoop: unknown }>);
vi.mock('three', async (original) => {
  const actual = await original<typeof import('three')>();
  class Renderer {
    domElement = document.createElement('canvas'); renders = 0; disposed = false; lost = false; animationLoop: unknown = null;
    renderLists = { dispose: vi.fn() };
    constructor() { instances.push(this); }
    getContext() { return { isContextLost: () => false }; }
    setSize() {} setPixelRatio() {}
    render() { this.renders += 1; }
    setAnimationLoop(value: unknown) { this.animationLoop = value; }
    dispose() { this.disposed = true; } forceContextLoss() { this.lost = true; }
  }
  return { ...actual, WebGLRenderer: Renderer };
});
import { createOwnerScene } from './scene';

const graph = () => { const projected = projectOwnerGraph(ownerGraphFixture()); return ownerGraphWindow(projected, projected.nodes); };
function setup() {
  const frames = new Map<number, FrameRequestCallback>(); let id = 0;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { frames.set(++id, callback); return id; });
  vi.stubGlobal('cancelAnimationFrame', (value: number) => frames.delete(value));
  return { frames, flush: () => { const current = [...frames.values()]; frames.clear(); current.forEach((callback) => callback(performance.now())); } };
}
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); instances.length = 0; document.body.replaceChildren(); });
describe('Owner scene demand rendering and disposal', () => {
  it('renders once per requested change, remains idle, and pauses hidden/offscreen', () => {
    const scheduler = setup(); let hidden = false; vi.spyOn(document, 'hidden', 'get').mockImplementation(() => hidden);
    const onlabels = vi.fn(); const container = document.createElement('div'); document.body.append(container);
    const scene = createOwnerScene({ container, graph: graph(), onselect: vi.fn(), onlabels, onunavailable: vi.fn() });
    scheduler.flush(); const renderer = instances[0]!;
    expect(renderer.renders).toBe(1); expect(renderer.animationLoop).toBeNull(); expect(scheduler.frames.size).toBe(0);
    scene.setSelected('scenario:scenario-a:1'); scheduler.flush(); expect(renderer.renders).toBe(2);
    scene.setVisible(false); scene.zoom(1.2); scheduler.flush(); expect(renderer.renders).toBe(2);
    scene.setVisible(true); scheduler.flush(); expect(renderer.renders).toBe(3);
    hidden = true; document.dispatchEvent(new Event('visibilitychange')); scene.zoom(1.2); scheduler.flush(); expect(renderer.renders).toBe(3);
    hidden = false; document.dispatchEvent(new Event('visibilitychange')); scheduler.flush(); expect(renderer.renders).toBe(4);
    expect(onlabels).toHaveBeenCalled(); scene.destroy(); expect(scheduler.frames.size).toBe(0);
  });
  it('disposes shared geometry, each material, canvas, controls and listeners over 20 cycles', () => {
    const scheduler = setup(); const geometry = vi.spyOn(BufferGeometry.prototype, 'dispose'); const material = vi.spyOn(Material.prototype, 'dispose');
    const add = vi.spyOn(document, 'addEventListener'); const remove = vi.spyOn(document, 'removeEventListener');
    for (let cycle = 0; cycle < 20; cycle += 1) {
      const container = document.createElement('div'); document.body.append(container);
      const scene = createOwnerScene({ container, graph: graph(), onselect: vi.fn(), onlabels: vi.fn(), onunavailable: vi.fn() });
      scheduler.flush(); scene.destroy(); scene.destroy(); expect(container.querySelector('canvas')).toBeNull();
    }
    expect(instances).toHaveLength(20); expect(instances.every((renderer) => renderer.disposed && renderer.lost && renderer.animationLoop === null)).toBe(true);
    expect(geometry).toHaveBeenCalledTimes(40); expect(material).toHaveBeenCalledTimes(100);
    expect(add.mock.calls.filter(([event]) => event === 'visibilitychange')).toHaveLength(20);
    expect(remove.mock.calls.filter(([event]) => event === 'visibilitychange')).toHaveLength(20);
    expect(scheduler.frames.size).toBe(0);
  });
  it('keeps concurrent instances independent, releases replaced geometry, and reports context loss', () => {
    const scheduler = setup(); const unavailable = vi.fn();
    const first = createOwnerScene({ container: document.createElement('div'), graph: graph(), onselect: vi.fn(), onlabels: vi.fn(), onunavailable: unavailable });
    const second = createOwnerScene({ container: document.createElement('div'), graph: graph(), onselect: vi.fn(), onlabels: vi.fn(), onunavailable: vi.fn() });
    scheduler.flush(); first.setSelected('evidence:run-a:scenario-a:1');
    first.update({ ...graph(), nodes: [], edges: [] }); scheduler.flush(); first.destroy();
    second.zoom(1.2); scheduler.flush(); expect(instances[1]!.disposed).toBe(false);
    second.canvas.dispatchEvent(new Event('webglcontextlost', { cancelable: true }));
    expect(scheduler.frames.size).toBe(0); second.destroy();
    expect(instances.every((renderer) => renderer.disposed)).toBe(true);
    expect(unavailable).not.toHaveBeenCalled();
  });
});
