import { render, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { EntityRef, EntityType, SystemMapData } from '../../lib/tauri/bindings';
import { SystemMapStore } from '../../lib/stores/systemMap.svelte';
import SystemMapScreen from './SystemMapScreen.svelte';

const emptyMap = (focus: EntityRef | null): SystemMapData => ({
  nodes: [], edges: [], focus, truncated: false, total_nodes_available: 0, dropped_edges: 0,
  kind_availability: [], focused_recent_events: [],
});

describe('SystemMapScreen query navigation', () => {
  beforeEach(() => {
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  });
  afterEach(() => {
    history.replaceState(null, '', '#/');
    vi.restoreAllMocks();
  });

  it('re-queries when search changes focus while the System Map route is already mounted', async () => {
    const fetch = vi.fn(async (focus: EntityRef | null, _depth: number, _kinds: EntityType[]) => emptyMap(focus));
    const controller = new SystemMapStore({ fetch });
    history.replaceState(null, '', '#/system-map?focus=service%3Aold&type=service&depth=2');
    render(SystemMapScreen, { controller });
    await waitFor(() => expect(fetch).toHaveBeenCalledWith({ entity_id: 'service:old', entity_type: 'service' }, 2, []));

    window.location.hash = '#/system-map?focus=service%3Anew&type=service&depth=3';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    await waitFor(() => expect(fetch).toHaveBeenLastCalledWith({ entity_id: 'service:new', entity_type: 'service' }, 3, []));
    expect(controller.focus).toEqual({ entity_id: 'service:new', entity_type: 'service' });
  });
});
