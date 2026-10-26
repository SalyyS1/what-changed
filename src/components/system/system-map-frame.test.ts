import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SystemMapData } from '../../lib/tauri/bindings';
import { SystemMapStore } from '../../lib/stores/systemMap.svelte';
import { eventDetails } from '../../lib/stores/eventDetails.svelte';
import { empty, ready } from '../../lib/types/async-state';
import SystemMapFrame from './SystemMapFrame.svelte';

class ResizeObserverStub {
  constructor(private callback: ResizeObserverCallback) {}
  observe(): void { this.callback([{ contentRect: { width: 800, height: 500 } } as ResizeObserverEntry], this as unknown as ResizeObserver); }
  disconnect(): void {}
  unobserve(): void {}
}

function fixture(count: number): SystemMapData {
  const nodes = Array.from({ length: count }, (_, index) => ({
    entity_id: `process:${index}`, entity_type: 'process' as const, display_name: `Process ${index}`,
    hop: index === 0 ? 0 : 1, degree: count - index, last_changed_at: null, recent_event_count: index,
  }));
  return {
    nodes,
    edges: nodes.slice(1).map((node, index) => ({ source: nodes[0]!.entity_id, target: node.entity_id, kind: 'provides', recorded_at: index, evidence_event_id: index + 1 })),
    focus: count > 0 ? { entity_id: nodes[0]!.entity_id, entity_type: 'process' } : null,
    truncated: false, total_nodes_available: count, dropped_edges: 0,
    kind_availability: [
      { kind: 'process', count, available: count > 0, reason: count > 0 ? null : 'No processes recorded' },
      { kind: 'container', count: 0, available: false, reason: 'No Docker containers recorded — the Docker collector is off' },
    ],
    focused_recent_events: count > 0 ? [{ event_id: 91, timestamp_ms: 1, summary: 'Process restarted', importance: 'notable' }] : [],
  };
}

describe('SystemMapFrame', () => {
  beforeEach(() => {
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);
    const context = { setTransform: vi.fn(), clearRect: vi.fn(), beginPath: vi.fn(), moveTo: vi.fn(), lineTo: vi.fn(), stroke: vi.fn(), arc: vi.fn(), fill: vi.fn() };
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(((kind: string) => kind === '2d' ? context : null) as never);
  });
  afterEach(() => vi.restoreAllMocks());

  it('falls back to the complete 2D surface and virtualizes its keyboard list', async () => {
    render(SystemMapFrame, { data: ready(fixture(100)), mode: '3d', controller: new SystemMapStore({ fetch: vi.fn() }) });
    await waitFor(() => expect(screen.getByText('2D')).toBeTruthy());
    const navigator = screen.getByRole('listbox', { name: 'Explore system map nodes' });
    expect(screen.getAllByRole('option').length).toBeLessThanOrEqual(41);
    await fireEvent.keyDown(navigator, { key: 'End' });
    expect(navigator.getAttribute('aria-activedescendant')).toBe('system-node-99');
  });

  it('names why a zero-count entity kind is disabled', async () => {
    render(SystemMapFrame, { data: ready(fixture(3)), mode: '2d', controller: new SystemMapStore({ fetch: vi.fn() }) });
    const chip = await screen.findByRole('button', { name: 'Containers, 0 recorded' });
    expect(chip.hasAttribute('disabled')).toBe(true);
    expect(chip.getAttribute('title')).toBe('No Docker containers recorded — the Docker collector is off');
  });

  it('shows the explicit relationship empty state without mounting a renderer', async () => {
    render(SystemMapFrame, { data: empty<SystemMapData>(), mode: '2d', controller: new SystemMapStore({ fetch: vi.fn() }) });
    expect(await screen.findByText('Nothing is connected yet.')).toBeTruthy();
    expect(screen.queryByTestId('system-map-2d')).toBeNull();
  });

  it('discloses both cap numbers', async () => {
    const data = fixture(40); data.truncated = true; data.total_nodes_available = 1832;
    render(SystemMapFrame, { data: ready(data), mode: '2d', controller: new SystemMapStore({ fetch: vi.fn() }) });
    expect(await screen.findByText((text) => text.startsWith('Showing 40 of') && text.endsWith('connected things'))).toBeTruthy();
  });

  it('opens an actual focused-node recent event in the shared drawer and restores focus to its row', async () => {
    const show = vi.spyOn(eventDetails, 'show').mockResolvedValue(undefined);
    render(SystemMapFrame, { data: ready(fixture(3)), mode: '2d', controller: new SystemMapStore({ fetch: vi.fn() }) });
    const opener = await screen.findByRole('button', { name: /Process restarted/ });
    await fireEvent.click(opener);
    expect(show).toHaveBeenCalledWith(91, opener);
  });
});
