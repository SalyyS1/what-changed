import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { SystemMapData } from '../../lib/tauri/bindings';
import DependencyPanel from './DependencyPanel.svelte';

describe('DependencyPanel historical relationships', () => {
  it('does not present a closed port as a current listener', () => {
    const recordedAt = 1_700_000_000_000;
    const data: SystemMapData = {
      nodes: [
        { entity_id: 'process:demo', entity_type: 'process', display_name: 'Demo API', hop: 0, degree: 1, last_changed_at: null, recent_event_count: 1 },
        { entity_id: 'port:tcp:0.0.0.0:3000', entity_type: 'port', display_name: 'TCP port 3000', hop: 1, degree: 1, last_changed_at: null, recent_event_count: 1 },
      ],
      edges: [{ source: 'process:demo', target: 'port:tcp:0.0.0.0:3000', kind: 'listens_on', recorded_at: recordedAt, evidence_event_id: 7 }],
      focus: { entity_id: 'process:demo', entity_type: 'process' },
      truncated: false,
      total_nodes_available: 2,
      dropped_edges: 0,
      kind_availability: [],
      focused_recent_events: [{ event_id: 8, timestamp_ms: recordedAt + 60_000, summary: 'TCP port 3000 stopped listening', importance: 'notable' }],
    };

    render(DependencyPanel, { data, onfocus: vi.fn() });

    expect(screen.getByText('These links record past observations, not a live connection or a causal claim.')).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Recorded outgoing links · 1' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'was observed listening on TCP port 3000' })).toBeTruthy();
    expect(screen.getByRole('button', { name: /TCP port 3000 stopped listening/ })).toBeTruthy();
    const time = screen.getByTitle('When this relationship was observed');
    expect(time.getAttribute('datetime')).toBe(new Date(recordedAt).toISOString());
  });
});
