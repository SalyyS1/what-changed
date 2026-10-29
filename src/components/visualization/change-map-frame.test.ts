import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ChangeMapData } from '../../lib/tauri/bindings';
import { eventDetails } from '../../lib/stores/eventDetails.svelte';
import { empty, ready } from '../../lib/types/async-state';
import ChangeMapFrame from './ChangeMapFrame.svelte';

class ResizeObserverStub {
  constructor(private callback: ResizeObserverCallback) {}
  observe(): void { this.callback([{ contentRect: { width: 800, height: 500 } } as ResizeObserverEntry], this as unknown as ResizeObserver); }
  disconnect(): void {}
  unobserve(): void {}
}

function fixture(count: number): ChangeMapData {
  return {
    nodes: Array.from({ length: count }, (_, index) => ({
      id: `event:${index + 1}`, event_id: index + 1, timestamp_ms: index,
      category: 'process', lane: 'process', importance: 'notable', label: `Change ${index + 1}`,
      chain_ids: index < 3 ? ['chain:1'] : [], chain_depth: index < 3 ? index : 0,
      is_suspect: index === 0, snapshot_only: false,
    })),
    edges: [], symptom_time_ms: count, window_start_ms: 0, window_end_ms: Math.max(1, count),
    lanes: [{ id: 'process', label: 'Process' }], truncated: false,
    total_nodes: count, omitted_nodes: 0, total_edges: 0, omitted_edges: 0,
  };
}

describe('ChangeMapFrame', () => {
  beforeEach(() => {
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);
    const context = {
      setTransform: vi.fn(), clearRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(),
      stroke: vi.fn(), arc: vi.fn(), fill: vi.fn(), setLineDash: vi.fn(),
    };
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(((kind: string) => kind === '2d' ? context : null) as never);
  });
  afterEach(() => vi.restoreAllMocks());

  it('falls back to 2D without logging when WebGL creation fails', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    render(ChangeMapFrame, {
      request: { kind: 'investigation', investigation_id: 1 }, subtitle: 'No WebGL',
      onclose: vi.fn(), data: ready(fixture(3)), mode: '3d',
    });
    await waitFor(() => expect(screen.getByText('2D')).toBeTruthy());
    expect(consoleError).not.toHaveBeenCalled();
  });

  it('ships an equivalent forced-2D surface with one keyboard entry point', async () => {
    render(ChangeMapFrame, {
      request: { kind: 'investigation', investigation_id: 1 }, subtitle: 'API cannot connect',
      onclose: vi.fn(), data: ready(fixture(3000)), mode: '2d',
    });
    const navigator = await screen.findByRole('listbox', { name: 'Explore map nodes' });
    expect(screen.getByText('2D')).toBeTruthy();
    expect(screen.getAllByRole('option').length).toBeLessThanOrEqual(41);
    await fireEvent.keyDown(navigator, { key: 'End' });
    expect(navigator.getAttribute('aria-activedescendant')).toBe('map-node-3000');
  });

  it('renders the explicit empty state without creating a renderer', async () => {
    render(ChangeMapFrame, {
      request: { kind: 'investigation', investigation_id: 1 }, subtitle: 'Empty',
      onclose: vi.fn(), data: empty<ChangeMapData>(), mode: '2d',
    });
    await waitFor(() => expect(screen.getByText('Nothing to map in this window.')).toBeTruthy());
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('opens the shared event drawer from the keyboard and supplies the list as the focus return target', async () => {
    const show = vi.spyOn(eventDetails, 'show').mockResolvedValue(undefined);
    render(ChangeMapFrame, {
      request: { kind: 'investigation', investigation_id: 1 }, subtitle: 'Keyboard',
      onclose: vi.fn(), data: ready(fixture(3)), mode: '2d',
    });
    const navigator = await screen.findByRole('listbox', { name: 'Explore map nodes' });
    navigator.focus();
    await fireEvent.keyDown(navigator, { key: 'Enter' });
    expect(show).toHaveBeenCalledWith(1, navigator);
  });

  it('opens retained snapshot details without fetching an expired source event', async () => {
    const data = fixture(3);
    data.nodes[0] = { ...data.nodes[0]!, snapshot_only: true };
    const show = vi.spyOn(eventDetails, 'show');
    const showSnapshot = vi.spyOn(eventDetails, 'showSnapshot');
    render(ChangeMapFrame, {
      request: { kind: 'investigation', investigation_id: 1 }, subtitle: 'Historical',
      onclose: vi.fn(), data: ready(data), mode: '2d',
    });
    const navigator = await screen.findByRole('listbox', { name: 'Explore map nodes' });
    await fireEvent.keyDown(navigator, { key: 'Enter' });

    expect(showSnapshot).toHaveBeenCalledWith(data.nodes[0], navigator);
    expect(show).not.toHaveBeenCalled();
  });
});
