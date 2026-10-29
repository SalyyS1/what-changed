import { render, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ChangeMapData } from '../../lib/tauri/bindings';

const scene = vi.hoisted(() => ({
  canvas: document.createElement('canvas'), update: vi.fn(), resize: vi.fn(), render: vi.fn(),
  setVisible: vi.fn(), setSelected: vi.fn(), refreshPalette: vi.fn(), resetView: vi.fn(),
  projectSymptom: vi.fn(() => null), projectNode: vi.fn(() => null),
  frameTiming: vi.fn(() => ({ samples: 0, median_ms: 0 })), destroy: vi.fn(),
}));
const sceneOptions = vi.hoisted(() => [] as Array<{
  quality?: string;
  onMetrics?: (median: number) => void;
}>);

vi.mock('../../three/changeMap/createScene', () => ({
  createChangeMapScene: vi.fn((options) => {
    sceneOptions.push(options);
    return scene;
  }),
}));

import ChangeMap from './ChangeMap.svelte';

class ResizeObserverStub {
  observe(): void {}
  disconnect(): void {}
  unobserve(): void {}
}

const data: ChangeMapData = {
  nodes: [], edges: [], symptom_time_ms: null, window_start_ms: 0, window_end_ms: 1, lanes: [],
  truncated: false, total_nodes: 0, omitted_nodes: 0, total_edges: 0, omitted_edges: 0,
};

describe('ChangeMap theme integration', () => {
  beforeEach(() => {
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);
    scene.refreshPalette.mockClear();
    sceneOptions.length = 0;
  });
  afterEach(() => {
    document.documentElement.removeAttribute('data-theme');
    vi.restoreAllMocks();
  });

  it('rewrites instance colours after the resolved theme changes', async () => {
    const view = render(ChangeMap, {
      data, selectedId: null, resetKey: 0, onhover: vi.fn(), onselect: vi.fn(), oncontextlost: vi.fn(),
    });
    document.documentElement.dataset.theme = 'light';
    await waitFor(() => expect(scene.refreshPalette).toHaveBeenCalled());
    view.unmount();
    expect(scene.destroy).toHaveBeenCalled();
  });

  it('rebuilds at low quality after a slow animation-frame median', async () => {
    const view = render(ChangeMap, {
      data, selectedId: null, resetKey: 0, onhover: vi.fn(), onselect: vi.fn(), oncontextlost: vi.fn(),
    });
    await waitFor(() => expect(sceneOptions).toHaveLength(1));
    expect(sceneOptions[0]?.quality).toBe('high');

    sceneOptions[0]?.onMetrics?.(20);

    await waitFor(() => expect(sceneOptions).toHaveLength(2));
    expect(sceneOptions[1]?.quality).toBe('low');
    view.unmount();
  });
});
