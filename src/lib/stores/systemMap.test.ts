import { describe, expect, it, vi } from 'vitest';
import type { SystemMapData } from '../tauri/bindings';
import { parseSystemMapHash, SystemMapStore, systemMapHash } from './systemMap.svelte';

const data = (id: string): SystemMapData => ({
  nodes: [{ entity_id: id, entity_type: 'process', display_name: id, hop: 0, degree: 1, last_changed_at: null, recent_event_count: 1 }],
  edges: [], focus: { entity_id: id, entity_type: 'process' }, truncated: false,
  total_nodes_available: 1, dropped_edges: 0,
  kind_availability: [{ kind: 'process', count: 1, available: true, reason: null }],
  focused_recent_events: [{ event_id: 1, timestamp_ms: 1, summary: `${id} changed`, importance: 'notable' }],
});

describe('SystemMapStore', () => {
  it('round-trips deterministic URL state', () => {
    const hash = systemMapHash({ focus: { entity_id: 'process:0123456789abcdef', entity_type: 'process' }, depth: 3, kinds: ['port', 'app'] });
    expect(parseSystemMapHash(hash)).toEqual({
      focus: { entity_id: 'process:0123456789abcdef', entity_type: 'process' },
      depth: 3,
      kinds: ['app', 'port'],
    });
  });

  it('discards malformed or stale entity types and keeps only allowlisted kinds', () => {
    expect(parseSystemMapHash('#/system-map?focus=application%3Aold&type=application&kind=service&kind=bogus&kind=service')).toEqual({
      focus: null,
      depth: 2,
      kinds: ['service'],
    });
    expect(parseSystemMapHash('#/system-map?focus=service%3Aapi&type=process&kind=port')).toEqual({
      focus: null,
      depth: 2,
      kinds: ['port'],
    });
  });

  it('accepts canonical port ids but rejects empty and malformed canonical suffixes', () => {
    expect(parseSystemMapHash('#/system-map?focus=port%3Atcp%3A0.0.0.0%3A3000&type=port').focus).toEqual({
      entity_id: 'port:tcp:0.0.0.0:3000', entity_type: 'port',
    });
    expect(parseSystemMapHash('#/system-map?focus=service%3A&type=service').focus).toBeNull();
    expect(parseSystemMapHash('#/system-map?focus=process%3Anot-a-hash&type=process').focus).toBeNull();
  });

  it('accepts Docker names and the volume/network entity prefixes', () => {
    expect(parseSystemMapHash('#/system-map?focus=container%3Awhat-changed-api&type=container').focus).toEqual({
      entity_id: 'container:what-changed-api', entity_type: 'container',
    });
    expect(parseSystemMapHash('#/system-map?focus=docker_volume%3Apostgres-data&type=docker_volume').focus).toEqual({
      entity_id: 'docker_volume:postgres-data', entity_type: 'docker_volume',
    });
    expect(parseSystemMapHash('#/system-map?focus=docker_network%3Aapp-default&type=docker_network').focus).toEqual({
      entity_id: 'docker_network:app-default', entity_type: 'docker_network',
    });
  });

  it('drops an older response after focus changes', async () => {
    let release: ((value: SystemMapData) => void) | undefined;
    const fetch = vi.fn()
      .mockReturnValueOnce(new Promise<SystemMapData>((resolve) => { release = resolve; }))
      .mockResolvedValueOnce(data('new'));
    const store = new SystemMapStore({ fetch });
    const old = store.load();
    store.focus = { entity_id: 'new', entity_type: 'process' };
    expect(await store.load()).toBe(true);
    release?.(data('old'));
    expect(await old).toBe(false);
    expect(store.state.status === 'ready' && store.state.data.focus?.entity_id).toBe('new');
  });
});
