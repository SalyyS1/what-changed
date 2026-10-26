import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import type { ChangeMapData } from '../../lib/tauri/bindings';
import ChangeMap2D from './ChangeMap2D.svelte';

class ResizeObserverStub {
  constructor(private callback: ResizeObserverCallback) {}
  observe(): void { this.callback([{ contentRect: { width: 800, height: 500 } } as ResizeObserverEntry], this as unknown as ResizeObserver); }
  disconnect(): void {}
  unobserve(): void {}
}

const data: ChangeMapData = {
  nodes: [{
    id: 'event:1', event_id: 1, timestamp_ms: 1, category: 'process', lane: 'process',
    importance: 'important', label: 'Process changed', chain_ids: ['chain:1'], chain_depth: 0,
    is_suspect: true, snapshot_only: false,
  }],
  edges: [], symptom_time_ms: 1, window_start_ms: 0, window_end_ms: 2,
  lanes: [{ id: 'process', label: 'Process' }], truncated: false,
  total_nodes: 1, omitted_nodes: 0, total_edges: 0, omitted_edges: 0,
};

let context: {
  setTransform: ReturnType<typeof vi.fn>; clearRect: ReturnType<typeof vi.fn>; beginPath: ReturnType<typeof vi.fn>;
  moveTo: ReturnType<typeof vi.fn>; lineTo: ReturnType<typeof vi.fn>; stroke: ReturnType<typeof vi.fn>;
  arc: ReturnType<typeof vi.fn>; fill: ReturnType<typeof vi.fn>; setLineDash: ReturnType<typeof vi.fn>;
};

describe('ChangeMap2D interactions', () => {
  beforeEach(() => {
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);
    context = {
      setTransform: vi.fn(), clearRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(),
      stroke: vi.fn(), arc: vi.fn(), fill: vi.fn(), setLineDash: vi.fn(),
    };
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(context as unknown as CanvasRenderingContext2D);
    vi.spyOn(HTMLCanvasElement.prototype, 'getBoundingClientRect').mockReturnValue({
      x: 0, y: 0, left: 0, top: 0, right: 800, bottom: 500, width: 800, height: 500, toJSON: () => ({}),
    });
    Object.defineProperty(HTMLCanvasElement.prototype, 'setPointerCapture', { configurable: true, value: vi.fn() });
  });
  afterEach(() => vi.restoreAllMocks());

  it('supports hover and click selection without selecting after a pan gesture', async () => {
    const onhover = vi.fn();
    const onselect = vi.fn();
    render(ChangeMap2D, { data, selectedId: null, resetKey: 0, onhover, onselect });
    await tick();
    const canvas = document.querySelector('canvas')!;
    const [x, y] = context.arc.mock.calls.at(-1) as [number, number];
    await fireEvent.pointerMove(canvas, { clientX: x, clientY: y, pointerId: 1 });
    expect(onhover).toHaveBeenLastCalledWith(data.nodes[0], x, y);
    await fireEvent.pointerDown(canvas, { clientX: x, clientY: y, pointerId: 1 });
    await fireEvent.pointerUp(canvas, { clientX: x, clientY: y, pointerId: 1 });
    expect(onselect).toHaveBeenCalledOnce();
    await fireEvent.pointerDown(canvas, { clientX: x, clientY: y, pointerId: 2 });
    await fireEvent.pointerMove(canvas, { clientX: x + 30, clientY: y, pointerId: 2 });
    await fireEvent.pointerUp(canvas, { clientX: x + 30, clientY: y, pointerId: 2 });
    expect(onselect).toHaveBeenCalledOnce();
  });

  it('redraws its token colours when the theme changes', async () => {
    const view = render(ChangeMap2D, { data, selectedId: null, resetKey: 0, onhover: vi.fn(), onselect: vi.fn() });
    await tick();
    const priorDraws = context.arc.mock.calls.length;
    document.documentElement.dataset.theme = 'light';
    await waitFor(() => expect(context.arc.mock.calls.length).toBeGreaterThan(priorDraws));
    view.unmount();
    document.documentElement.removeAttribute('data-theme');
  });
});
