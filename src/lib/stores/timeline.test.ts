import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BatchNotice, EventSummary, TimelinePage } from '../tauri/bindings';
import { MAX_PAGES, PAGE_SIZE, TimelineStore } from './timeline.svelte';
import { clusterIdentity, timelineItemIdentity } from '../timeline/identity';

function event(id: number, timestamp = 1_700_000_000_000 - id): EventSummary {
  return { id, timestamp, category: 'process', event_type: 'started', entity_type: 'process', entity_id: `process:${id}`, entity_display_name: `app-${id}.exe`, severity: 'info', importance: 'informational', confidence: 'high', source_collector: 'synthetic', before: null, after: null, field: null, user_friendly_summary: `app-${id}.exe started`, occurred_at_is_uncertain: false };
}

function page(id: number, next: string | null): TimelinePage {
  return { items: [{ kind: 'event', event: event(id) }], next };
}

function fullPage(from: number, next: string | null): TimelinePage {
  return {
    items: Array.from({ length: PAGE_SIZE }, (_, index) => ({
      kind: 'event' as const,
      event: event(from + index, 1_700_000_000_000),
    })),
    next,
  };
}

function cluster(firstId: number, lastId: number) {
  const reference = {
    key: 'process:api:28333333',
    category: 'process' as const,
    coarse_entity: 'api',
    bucket: 28_333_333,
    first_ts: 1_700_000_000_000,
    first_id: firstId,
    last_ts: 1_700_000_000_000,
    last_id: lastId,
  };
  return {
    kind: 'cluster' as const,
    key: reference.key,
    cluster: reference,
    label: `${lastId - firstId + 1} API changes`,
    count: lastId - firstId + 1,
    first_ts: reference.first_ts,
    last_ts: reference.last_ts,
    highest_importance: 'notable' as const,
    category: 'process' as const,
  };
}

const notice: BatchNotice = { count: 2, newest_id: 99, event_ids: [], categories: ['process'], highest_importance: 'informational' };

describe('TimelineStore', () => {
  beforeEach(() => { window.location.hash = '#/timeline'; });

  it('combines validated range/filter/entity state into a bounded query and hash', async () => {
    const fetchPage = vi.fn().mockResolvedValue(page(1, null));
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.setRange('last7_days');
    await store.applyFilter('drivers');
    await store.filterToEntity('driver:nvidia');

    expect(fetchPage).toHaveBeenLastCalledWith(expect.objectContaining({
      range: { kind: 'last7_days' }, categories: [], entity_id: 'driver:nvidia', limit: PAGE_SIZE,
    }));
    expect(window.location.hash).toContain('range=last7_days');
    expect(window.location.hash).toContain('entity=driver%3Anvidia');
  });

  it('rejects fractional, negative, and unsafe custom timestamps from calls and hashes', async () => {
    const fetchPage = vi.fn().mockResolvedValue(page(1, null));
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    const originalFrom = store.customFrom;
    await store.setCustomRange(1.5, 2);
    await store.setCustomRange(-1, 2);
    await store.setCustomRange(Number.MAX_SAFE_INTEGER + 1, Number.MAX_SAFE_INTEGER + 2);
    expect(fetchPage).not.toHaveBeenCalled();
    expect(store.customFrom).toBe(originalFrom);

    store.restoreFromHash('#/timeline?range=custom&from=1.5&to=2');
    expect(store.customFrom).toBe(originalFrom);
  });

  it('preserves the contextual map flag while synchronizing filters', async () => {
    window.location.hash = '#/timeline?view=map&range=today&filter=all';
    const store = new TimelineStore({
      fetchPage: vi.fn().mockResolvedValue(page(1, null)),
      fetchCluster: vi.fn(),
    });
    await store.applyFilter('network');

    expect(window.location.hash).toContain('view=map');
    expect(window.location.hash).toContain('filter=network');
  });

  it('keeps at most three pages / 600 raw rows', async () => {
    const fetchPage = vi.fn()
      .mockResolvedValueOnce(page(1, 'a'))
      .mockResolvedValueOnce(page(2, 'b'))
      .mockResolvedValueOnce(page(3, 'c'))
      .mockResolvedValueOnce(page(4, null));
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.load(); await store.loadMore(); await store.loadMore(); await store.loadMore();

    expect(store.state.status).toBe('ready');
    if (store.state.status === 'ready') {
      expect(store.state.data.items).toHaveLength(MAX_PAGES);
      expect(store.state.data.loadedRows).toBeLessThanOrEqual(MAX_PAGES * PAGE_SIZE);
    }
  });

  it('does not move the viewport when live events arrive away from the top', async () => {
    const fetchPage = vi.fn().mockResolvedValue(page(1, null));
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.load();
    store.setAtTop(false);
    await store.handleLiveNotice(notice);

    expect(store.newCount).toBe(2);
    expect(fetchPage).toHaveBeenCalledTimes(1);
  });

  it('prepends only new rows for a live range at scroll top', async () => {
    const fetchPage = vi.fn().mockResolvedValueOnce(page(1, null)).mockResolvedValueOnce({
      items: [{ kind: 'event', event: event(2) }, { kind: 'event', event: event(1) }], next: null,
    });
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.load();
    await store.handleLiveNotice(notice);
    expect(store.state.status === 'ready' && store.state.data.items.map((item) => item.kind === 'event' ? item.event.id : 0)).toEqual([2, 1]);
  });

  it('keeps the 600-row ceiling when a full first page refreshes live', async () => {
    const fetchPage = vi.fn()
      .mockResolvedValueOnce(fullPage(1, 'a'))
      .mockResolvedValueOnce(fullPage(201, 'b'))
      .mockResolvedValueOnce(fullPage(401, 'c'))
      .mockResolvedValueOnce(fullPage(1_001, 'fresh'));
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.load(); await store.loadMore(); await store.loadMore();
    await store.handleLiveNotice(notice);

    // A live refresh intentionally invalidates the old downstream cursor
    // chain; only the authoritative first page remains until loadMore.
    expect(store.state.status === 'ready' && store.state.data.loadedRows).toBe(PAGE_SIZE);
    expect(store.cachedEventCount).toBeLessThanOrEqual(MAX_PAGES * PAGE_SIZE);
  });

  it('restarts the cursor chain after a live refresh so displaced boundary rows remain reachable', async () => {
    const fresh = {
      items: [
        ...Array.from({ length: 10 }, (_, index) => ({ kind: 'event' as const, event: event(1_000 + index) })),
        ...Array.from({ length: 190 }, (_, index) => ({ kind: 'event' as const, event: event(1 + index) })),
      ],
      next: 'fresh-boundary',
    };
    const displaced: TimelinePage = {
      // Deliberately repeat the cursor boundary row; the UI must not expose a
      // duplicate even if a backend snapshot overlaps by one.
      items: [
        { kind: 'event' as const, event: event(190) },
        ...Array.from({ length: 199 }, (_, index) => ({ kind: 'event' as const, event: event(191 + index) })),
      ],
      next: null,
    };
    const fetchPage = vi.fn()
      .mockResolvedValueOnce(fullPage(1, 'old-boundary'))
      .mockResolvedValueOnce(fullPage(201, null))
      .mockResolvedValueOnce(fresh)
      .mockResolvedValueOnce(displaced);
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.load();
    await store.loadMore();
    await store.handleLiveNotice(notice);
    await store.loadMore();

    expect(fetchPage.mock.calls[3]?.[0].cursor).toBe('fresh-boundary');
    if (store.state.status === 'ready') {
      const ids = store.state.data.items.flatMap((item) => item.kind === 'event' ? [item.event.id] : []);
      expect(ids).toContain(191);
      expect(new Set(ids).size).toBe(ids.length);
    }
  });

  it('queues a live pill instead of racing an in-flight pagination cursor', async () => {
    let resolveMore!: (value: TimelinePage) => void;
    const pendingMore = new Promise<TimelinePage>((resolve) => { resolveMore = resolve; });
    const fetchPage = vi.fn()
      .mockResolvedValueOnce(page(1, 'old-boundary'))
      .mockReturnValueOnce(pendingMore);
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.load();
    const loading = store.loadMore();
    await store.handleLiveNotice(notice);

    expect(fetchPage).toHaveBeenCalledTimes(2);
    expect(store.newCount).toBe(notice.count);
    resolveMore(page(2, null));
    await loading;
    expect(store.state.status === 'ready' && store.state.data.items).toHaveLength(2);
  });

  it('refetches a newer page after the three-page window evicts it', async () => {
    const fetchPage = vi.fn()
      .mockResolvedValueOnce(page(1, 'a'))
      .mockResolvedValueOnce(page(2, 'b'))
      .mockResolvedValueOnce(page(3, 'c'))
      .mockResolvedValueOnce(page(4, null))
      .mockResolvedValueOnce(page(1, 'a'))
      .mockResolvedValueOnce(page(2, 'b'));
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.load(); await store.loadMore(); await store.loadMore(); await store.loadMore();
    expect(store.state.status === 'ready' && store.state.data.hasNewer).toBe(true);
    await store.loadNewer();

    expect(fetchPage.mock.calls[4]?.[0].cursor).toBeNull();
    if (store.state.status === 'ready') {
      expect(store.state.data.items.flatMap((item) => item.kind === 'event' ? [item.event.id] : [])).toEqual([1]);
      expect(store.state.data.hasNewer).toBe(false);
    }
    await store.loadMore();
    expect(fetchPage.mock.calls[5]?.[0].cursor).toBe('a');
    expect(store.state.status === 'ready' && store.state.data.items.flatMap((item) => item.kind === 'event' ? [item.event.id] : [])).toEqual([1, 2]);
  });

  it('does not splice a freshly shifted page zero into stale retained pages', async () => {
    const fresh: TimelinePage = {
      items: [
        ...Array.from({ length: 10 }, (_, index) => ({ kind: 'event' as const, event: event(1_000 + index) })),
        ...Array.from({ length: 190 }, (_, index) => ({ kind: 'event' as const, event: event(1 + index) })),
      ],
      next: 'fresh-boundary',
    };
    const nextFresh: TimelinePage = {
      items: [
        { kind: 'event' as const, event: event(190) },
        ...Array.from({ length: 199 }, (_, index) => ({ kind: 'event' as const, event: event(191 + index) })),
      ],
      next: null,
    };
    const fetchPage = vi.fn()
      .mockResolvedValueOnce(fullPage(1, 'old-a'))
      .mockResolvedValueOnce(fullPage(201, 'old-b'))
      .mockResolvedValueOnce(fullPage(401, 'old-c'))
      .mockResolvedValueOnce(fullPage(601, null))
      .mockResolvedValueOnce(fresh)
      .mockResolvedValueOnce(nextFresh);
    const store = new TimelineStore({ fetchPage, fetchCluster: vi.fn() });
    await store.load(); await store.loadMore(); await store.loadMore(); await store.loadMore();
    expect(store.state.status === 'ready' && store.state.data.loadedRows).toBe(600);

    await store.loadNewer();
    expect(fetchPage.mock.calls[4]?.[0].cursor).toBeNull();
    expect(store.state.status === 'ready' && store.state.data.loadedRows).toBe(PAGE_SIZE);
    await store.loadMore();
    expect(fetchPage.mock.calls[5]?.[0].cursor).toBe('fresh-boundary');

    if (store.state.status === 'ready') {
      const ids = store.state.data.items.flatMap((item) => item.kind === 'event' ? [item.event.id] : []);
      expect(ids).toHaveLength(399);
      expect(new Set(ids).size).toBe(ids.length);
      for (let id = 1; id <= 389; id += 1) expect(ids).toContain(id);
      expect(store.state.data.loadedRows).toBeLessThanOrEqual(MAX_PAGES * PAGE_SIZE);
    }
  });

  it('keeps same-minute clusters distinct across pages and live boundary changes', async () => {
    // More than one page of events can share the same minute/key. Exact
    // boundaries, not the semantic key alone, keep all 320 rows addressable.
    const first = cluster(1, 160);
    const second = cluster(161, 320);
    const changed = cluster(1, 161);
    const fetchPage = vi.fn()
      .mockResolvedValueOnce({ items: [first], next: 'next-minute-slice' })
      .mockResolvedValueOnce({ items: [second], next: null })
      .mockResolvedValueOnce({ items: [changed], next: null });
    const fetchCluster = vi.fn()
      .mockResolvedValueOnce([event(1)])
      .mockResolvedValueOnce([event(161)])
      .mockResolvedValueOnce([event(1001)]);
    const store = new TimelineStore({ fetchPage, fetchCluster });
    await store.load(); await store.loadMore();
    await store.expand(first.cluster); await store.expand(second.cluster);

    expect(clusterIdentity(first.cluster)).not.toBe(clusterIdentity(second.cluster));
    if (store.state.status === 'ready') {
      expect(new Set(store.state.data.items.map(timelineItemIdentity)).size).toBe(store.state.data.items.length);
    }
    await store.handleLiveNotice(notice);
    expect(store.entering.has(timelineItemIdentity(changed))).toBe(true);
    expect(store.expandedKeys.size).toBe(0);
    await store.expand(changed.cluster);
    expect(fetchCluster).toHaveBeenCalledTimes(3);
  });

  it('bounds expanded member payloads and prunes them when their page is evicted', async () => {
    const first = cluster(1, 2);
    const fetchPage = vi.fn()
      .mockResolvedValueOnce({ items: [first], next: 'a' })
      .mockResolvedValueOnce(fullPage(201, 'b'))
      .mockResolvedValueOnce(fullPage(401, 'c'))
      .mockResolvedValueOnce(fullPage(601, null));
    const fetchCluster = vi.fn().mockResolvedValue(Array.from({ length: 20 }, (_, index) => event(index + 1)));
    const store = new TimelineStore({ fetchPage, fetchCluster });
    await store.load(); await store.expand(first.cluster);
    expect(store.cachedEventCount).toBe(2);
    await store.loadMore(); await store.loadMore(); await store.loadMore();
    expect(store.expandedKeys.size).toBe(0);
    expect(store.cachedEventCount).toBeLessThanOrEqual(MAX_PAGES * PAGE_SIZE);
  });

  it('expands a cluster inline and collapses it without refetching', async () => {
    const cluster = { key: 'process:api:1', category: 'process' as const, coarse_entity: 'api', bucket: 1, first_ts: 1, first_id: 1, last_ts: 2, last_id: 2 };
    const fetchCluster = vi.fn().mockResolvedValue([event(1), event(2)]);
    const store = new TimelineStore({ fetchPage: vi.fn().mockResolvedValue({ items: [{ kind: 'cluster', key: cluster.key, cluster, label: '2 API changes', count: 2, first_ts: 1, last_ts: 2, highest_importance: 'notable', category: 'process' }], next: null }), fetchCluster });
    await store.load(); await store.expand(cluster);
    expect(store.state.status === 'ready' && store.state.data.items).toHaveLength(3);
    await store.expand(cluster);
    expect(store.state.status === 'ready' && store.state.data.items).toHaveLength(1);
    expect(fetchCluster).toHaveBeenCalledWith(cluster);
    expect(fetchCluster).toHaveBeenCalledTimes(1);
  });
});
