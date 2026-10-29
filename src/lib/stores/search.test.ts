import { afterEach, describe, expect, it, vi } from 'vitest';
import type { SearchResultDto } from '../tauri/bindings';
import { SearchStore } from './search.svelte';

const port: SearchResultDto = { kind: 'event', category: 'network', importance: 'notable', event: { id: 1, timestamp: 1, category: 'network', event_type: 'opened', entity_type: 'port', entity_id: 'port:25565', entity_display_name: '25565', severity: 'info', importance: 'notable', confidence: 'high', source_collector: 'synthetic', before: null, after: null, field: null, user_friendly_summary: 'Port 25565 opened', occurred_at_is_uncertain: false } };

describe('SearchStore', () => {
  afterEach(() => vi.useRealTimers());

  it('debounces by 120ms and groups a numeric port result', async () => {
    vi.useFakeTimers();
    const search = vi.fn().mockResolvedValue([port]);
    const store = new SearchStore({ search });
    store.setQuery('25565');
    await vi.advanceTimersByTimeAsync(119);
    expect(search).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(search).toHaveBeenCalledWith('25565', 100);
    await Promise.resolve();
    expect(store.state.status === 'ready' && store.state.data[0]?.name).toBe('Ports');
  });

  it('waits 300ms after dispatch before showing loading skeletons', async () => {
    vi.useFakeTimers();
    const store = new SearchStore({ search: vi.fn(() => new Promise<SearchResultDto[]>(() => {})) });
    store.setQuery('discord');

    expect(store.searching).toBe(true);
    await vi.advanceTimersByTimeAsync(419);
    expect(store.state.status).toBe('empty');
    await vi.advanceTimersByTimeAsync(1);
    expect(store.state.status).toBe('loading');
  });
});
