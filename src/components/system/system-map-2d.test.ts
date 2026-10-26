import { fireEvent, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import type { SystemMapData } from '../../lib/tauri/bindings';
import SystemMap2D from './SystemMap2D.svelte';

class ResizeObserverStub {
  constructor(private callback: ResizeObserverCallback) {}
  observe(): void { this.callback([{ contentRect: { width: 800, height: 500 } } as ResizeObserverEntry], this as unknown as ResizeObserver); }
  disconnect(): void {}
  unobserve(): void {}
}

const data: SystemMapData = {
  nodes: [{ entity_id: 'process:api', entity_type: 'process', display_name: 'API', hop: 0, degree: 2, last_changed_at: null, recent_event_count: 1 }],
  edges: [], focus: { entity_id: 'process:api', entity_type: 'process' }, truncated: false,
  total_nodes_available: 1, dropped_edges: 0,
  kind_availability: [{ kind: 'process', count: 1, available: true, reason: null }],
  focused_recent_events: [{ event_id: 1, timestamp_ms: 1, summary: 'API started', importance: 'notable' }],
};

let context: { arc: ReturnType<typeof vi.fn> } & Record<string, unknown>;

describe('SystemMap2D interactions', () => {
  beforeEach(() => {
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);
    context = { setTransform: vi.fn(), clearRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), stroke: vi.fn(), arc: vi.fn(), fill: vi.fn() };
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(context as unknown as CanvasRenderingContext2D);
    vi.spyOn(HTMLCanvasElement.prototype, 'getBoundingClientRect').mockReturnValue({ x: 0, y: 0, left: 0, top: 0, right: 800, bottom: 500, width: 800, height: 500, toJSON: () => ({}) });
    Object.defineProperty(HTMLCanvasElement.prototype, 'setPointerCapture', { configurable: true, value: vi.fn() });
  });
  afterEach(() => vi.restoreAllMocks());

  it('focuses a clicked node but suppresses focus after a pan', async () => {
    const onfocus = vi.fn(); const onhover = vi.fn();
    render(SystemMap2D, { data, selectedId: null, resetKey: 0, onhover, onfocus });
    await tick();
    const canvas = document.querySelector('canvas')!;
    const [x, y] = context.arc.mock.calls.at(-1) as [number, number];
    await fireEvent.pointerMove(canvas, { clientX: x, clientY: y, pointerId: 1 });
    expect(onhover).toHaveBeenLastCalledWith(data.nodes[0], x, y);
    await fireEvent.pointerDown(canvas, { clientX: x, clientY: y, pointerId: 1 });
    await fireEvent.pointerUp(canvas, { clientX: x, clientY: y, pointerId: 1 });
    expect(onfocus).toHaveBeenCalledWith(data.focus);
    await fireEvent.pointerDown(canvas, { clientX: x, clientY: y, pointerId: 2 });
    await fireEvent.pointerMove(canvas, { clientX: x + 30, clientY: y, pointerId: 2 });
    await fireEvent.pointerUp(canvas, { clientX: x + 30, clientY: y, pointerId: 2 });
    expect(onfocus).toHaveBeenCalledOnce();
    await fireEvent.pointerDown(canvas, { clientX: x, clientY: y, pointerId: 3 });
    await fireEvent.pointerCancel(canvas, { clientX: x, clientY: y, pointerId: 3 });
    expect(onhover).toHaveBeenLastCalledWith(null, 0, 0);
  });
});
