import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import type {
  BatchNotice,
  Category,
  ClusterRef,
  EventQuery,
  EventSummary,
  Importance,
  TimelineItem,
  TimelinePage,
  TimeRange,
} from '../tauri/bindings';
import { expandCluster, getEvents, toAppError } from '../tauri/client';
import { liveEvents } from './liveEvents.svelte';
import { clusterIdentity, timelineItemIdentity } from '../timeline/identity';

export const PAGE_SIZE = 200;
export const MAX_PAGES = 3;

export type TimelineRangeKey =
  | 'today'
  | 'yesterday'
  | 'last7_days'
  | 'since_last_restart'
  | 'custom';
export type TimelineFilterKey =
  | 'all'
  | 'apps'
  | 'windows'
  | 'network'
  | 'drivers'
  | 'files'
  | 'development'
  | 'docker'
  | 'important';

export interface TimelineSnapshot {
  items: TimelineItem[];
  loadedRows: number;
  totalCount: number | null;
  hasMore: boolean;
  hasNewer?: boolean;
}

interface PageWindow {
  index: number;
  cursor: string | null;
  items: TimelineItem[];
  next: string | null;
  rawCount: number;
}

interface TimelineDependencies {
  fetchPage: (query: EventQuery) => Promise<TimelinePage>;
  fetchCluster: (cluster: ClusterRef) => Promise<EventSummary[]>;
}

const defaultDependencies: TimelineDependencies = {
  fetchPage: getEvents,
  fetchCluster: expandCluster,
};

const filterCategories: Record<TimelineFilterKey, Category[]> = {
  all: [],
  apps: ['application'],
  windows: ['system', 'update', 'service', 'startup', 'security'],
  network: ['network'],
  drivers: ['driver', 'hardware'],
  files: ['filesystem', 'configuration'],
  development: ['process', 'runtime', 'git'],
  docker: ['docker'],
  important: [],
};

function rawCount(items: TimelineItem[]): number {
  return items.reduce((total, item) => total + (item.kind === 'cluster' ? item.count : 1), 0);
}

function withoutKnownItems(items: TimelineItem[], pages: PageWindow[]): TimelineItem[] {
  const known = new Set(pages.flatMap((page) => page.items.map(timelineItemIdentity)));
  return items.filter((item) => {
    const identity = timelineItemIdentity(item);
    if (known.has(identity)) return false;
    known.add(identity);
    return true;
  });
}

function parseRange(value: string | null): TimelineRangeKey {
  return ['today', 'yesterday', 'last7_days', 'since_last_restart', 'custom'].includes(value ?? '')
    ? (value as TimelineRangeKey)
    : 'today';
}

function parseFilter(value: string | null): TimelineFilterKey {
  return Object.hasOwn(filterCategories, value ?? '') ? (value as TimelineFilterKey) : 'all';
}

export class TimelineStore {
  state = $state<AsyncState<TimelineSnapshot>>(loading());
  range = $state<TimelineRangeKey>('today');
  filter = $state<TimelineFilterKey>('all');
  entityId = $state<string | null>(null);
  customFrom = $state(Date.now() - 24 * 60 * 60 * 1000);
  customTo = $state(Date.now());
  newCount = $state(0);
  loadingMore = $state(false);
  atTop = $state(true);
  entering = $state<Set<string>>(new Set());
  #pages = $state<PageWindow[]>([]);
  #expandedMembers = $state<Map<string, EventSummary[]>>(new Map());
  #cursorByIndex = new Map<number, string | null>();
  #windowStart = 0;
  #request = 0;
  #refreshingTop = false;
  #deps: TimelineDependencies;

  constructor(dependencies: TimelineDependencies = defaultDependencies) {
    this.#deps = dependencies;
  }

  get isLiveRange(): boolean {
    return this.range === 'today' || this.range === 'since_last_restart';
  }

  get expandedKeys(): ReadonlySet<string> {
    return new Set(this.#expandedMembers.keys());
  }

  /** Event payloads retained by the bounded window, including expansions. */
  get cachedEventCount(): number {
    const direct = this.#pages.reduce(
      (total, page) => total + page.items.filter((item) => item.kind === 'event').length,
      0,
    );
    return direct + [...this.#expandedMembers.values()].reduce((total, members) => total + members.length, 0);
  }

  get query(): EventQuery {
    return {
      range: this.#timeRange(),
      categories: filterCategories[this.filter],
      importance_min: this.filter === 'important' ? ('important' satisfies Importance) : null,
      entity_id: this.entityId,
      cursor: null,
      limit: PAGE_SIZE,
    };
  }

  restoreFromHash(hash = globalThis.location?.hash ?? ''): void {
    const query = hash.includes('?') ? hash.slice(hash.indexOf('?') + 1) : '';
    const params = new URLSearchParams(query);
    this.range = parseRange(params.get('range'));
    this.filter = parseFilter(params.get('filter'));
    const entity = params.get('entity');
    this.entityId = entity && entity.length <= 256 ? entity : null;
    const from = Number(params.get('from'));
    const to = Number(params.get('to'));
    if (this.range === 'custom' && validTimestamp(from) && validTimestamp(to) && from < to) {
      this.customFrom = from;
      this.customTo = to;
    }
  }

  async load(): Promise<void> {
    const request = ++this.#request;
    this.#refreshingTop = false;
    const loadingDelay = setTimeout(() => {
      if (request === this.#request) this.state = loading();
    }, 300);
    this.#pages = [];
    this.#expandedMembers = new Map();
    this.#cursorByIndex = new Map([[0, null]]);
    this.#windowStart = 0;
    this.loadingMore = false;
    this.newCount = 0;
    try {
      const page = await this.#deps.fetchPage(this.query);
      if (request !== this.#request) return;
      if (page.items.length === 0) {
        this.state = empty();
        return;
      }
      this.#installFirstPage(page);
      this.#publish();
    } catch (reason) {
      if (request !== this.#request) return;
      const error = toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => void this.load() : undefined);
    } finally {
      clearTimeout(loadingDelay);
    }
  }

  async loadMore(): Promise<void> {
    const cursor = this.#pages.at(-1)?.next;
    if (!cursor || this.loadingMore || this.#refreshingTop) return;
    const request = this.#request;
    this.loadingMore = true;
    try {
      const page = await this.#deps.fetchPage({ ...this.query, cursor });
      if (request !== this.#request) return;
      const items = withoutKnownItems(page.items, this.#pages);
      const index = this.#windowStart + this.#pages.length;
      this.#cursorByIndex.set(index, cursor);
      if (page.next !== null) this.#cursorByIndex.set(index + 1, page.next);
      const pages = [
        ...this.#pages,
        { index, cursor, items, next: page.next, rawCount: rawCount(items) },
      ];
      this.#pages = pages.slice(-MAX_PAGES);
      this.#windowStart = this.#pages[0]?.index ?? 0;
      this.#pruneExpandedMembers();
      this.#publish();
    } catch {
      // Keep the rows already on screen. A retry happens at the same threshold.
    } finally {
      if (request === this.#request) this.loadingMore = false;
    }
  }

  /** Refetch the adjacent newer page after the bounded window evicts it. */
  async loadNewer(): Promise<void> {
    if (this.#windowStart === 0 || this.loadingMore || this.#refreshingTop) return;
    const index = this.#windowStart - 1;
    const cursor = this.#cursorByIndex.get(index);
    if (cursor === undefined) return;
    const request = this.#request;
    this.loadingMore = true;
    try {
      const page = await this.#deps.fetchPage({ ...this.query, cursor });
      if (request !== this.#request) return;
      if (index === 0) {
        // The null cursor always means "the current first page". Inserts may
        // have shifted its lower boundary since the old page 1-3 window was
        // cached, so it cannot be safely prepended to that stale chain.
        // Install page 0 and its returned cursor atomically; loadMore then
        // follows the fresh chain and makes every displaced row reachable.
        this.#installFirstPage(page);
        this.#publish();
        return;
      }
      const items = withoutKnownItems(page.items, this.#pages);
      this.#pages = [
        { index, cursor, items, next: page.next, rawCount: rawCount(items) },
        ...this.#pages,
      ].slice(0, MAX_PAGES);
      this.#windowStart = index;
      this.#pruneExpandedMembers();
      this.#publish();
    } catch {
      // The retained window remains navigable; another upward scroll retries.
    } finally {
      if (request === this.#request) this.loadingMore = false;
    }
  }

  async setRange(range: TimelineRangeKey): Promise<void> {
    this.range = range;
    this.#syncHash();
    await this.load();
  }

  async setCustomRange(from: number, to: number): Promise<void> {
    if (!validTimestamp(from) || !validTimestamp(to) || from >= to) return;
    this.customFrom = from;
    this.customTo = to;
    await this.setRange('custom');
  }

  async applyFilter(filter: TimelineFilterKey): Promise<void> {
    this.filter = filter;
    this.entityId = null;
    this.#syncHash();
    await this.load();
  }

  async filterToEntity(entityId: string): Promise<void> {
    this.entityId = entityId.slice(0, 256);
    this.filter = 'all';
    this.#syncHash();
    await this.load();
  }

  setAtTop(value: boolean): void {
    this.atTop = value;
  }

  async handleLiveNotice(notice: BatchNotice): Promise<void> {
    if (!this.isLiveRange) return;
    if (
      !this.atTop ||
      this.#windowStart !== 0 ||
      this.loadingMore ||
      this.#refreshingTop
    ) {
      this.newCount += notice.count;
      return;
    }
    const request = ++this.#request;
    this.#refreshingTop = true;
    try {
      const page = await this.#deps.fetchPage(this.query);
      if (request !== this.#request) return;
      const known = new Set(this.#pages.flatMap((entry) => entry.items.map(timelineItemIdentity)));
      const additions = page.items.filter((item) => !known.has(timelineItemIdentity(item)));
      this.entering = new Set(additions.map(timelineItemIdentity));
      // The first page and its `next` cursor are an atomic snapshot. Keeping
      // any old downstream page would splice two cursor chains together and
      // could skip the rows displaced by the refresh boundary.
      // Replacing, rather than concatenating, also keeps the three-page
      // window at the hard 600-row ceiling.
      this.#installFirstPage(page);
      this.#publish();
      setTimeout(() => (this.entering = new Set()), 140);
    } catch {
      this.newCount += notice.count;
    } finally {
      this.#refreshingTop = false;
    }
  }

  async showNew(): Promise<void> {
    this.newCount = 0;
    this.atTop = true;
    await this.load();
  }

  async expand(cluster: ClusterRef): Promise<void> {
    const identity = clusterIdentity(cluster);
    if (this.#expandedMembers.has(identity)) {
      const next = new Map(this.#expandedMembers);
      next.delete(identity);
      this.#expandedMembers = next;
      this.#publish();
      return;
    }
    const request = this.#request;
    try {
      const members = await this.#deps.fetchCluster(cluster);
      if (request !== this.#request) return;
      if (!this.#visibleClusterIdentities().has(identity)) return;
      this.#expandedMembers = new Map(this.#expandedMembers).set(identity, members.slice(0, this.#clusterCount(identity)));
      this.#publish();
    } catch {
      // The cluster remains intact, so retrying it is safe.
    }
  }

  async attachLive(): Promise<() => void> {
    return liveEvents.attach((notice) => void this.handleLiveNotice(notice));
  }

  /** Install page zero and its continuation cursor as one coherent chain. */
  #installFirstPage(page: TimelinePage): void {
    this.#pages = [
      {
        index: 0,
        cursor: null,
        items: page.items,
        next: page.next,
        rawCount: rawCount(page.items),
      },
    ];
    this.#windowStart = 0;
    this.#cursorByIndex = new Map([[0, null]]);
    if (page.next !== null) this.#cursorByIndex.set(1, page.next);
    this.#pruneExpandedMembers();
  }

  #publish(): void {
    const items = this.#pages.flatMap((page) => page.items).flatMap((item) =>
      item.kind === 'cluster'
        ? [item, ...(this.#expandedMembers.get(clusterIdentity(item.cluster)) ?? []).map((event) => ({ kind: 'event' as const, event }))]
        : [item],
    );
    const loadedRows = this.#pages.reduce((total, page) => total + page.rawCount, 0);
    this.state = ready({
      items,
      loadedRows,
      totalCount: null,
      hasMore: this.#pages.at(-1)?.next !== null,
      hasNewer: this.#windowStart > 0,
    });
  }

  #visibleClusterIdentities(): Set<string> {
    return new Set(
      this.#pages.flatMap((page) =>
        page.items.flatMap((item) => item.kind === 'cluster' ? [clusterIdentity(item.cluster)] : []),
      ),
    );
  }

  #clusterCount(identity: string): number {
    for (const page of this.#pages) {
      const cluster = page.items.find(
        (item) => item.kind === 'cluster' && clusterIdentity(item.cluster) === identity,
      );
      if (cluster?.kind === 'cluster') return cluster.count;
    }
    return 0;
  }

  #pruneExpandedMembers(): void {
    const visible = this.#visibleClusterIdentities();
    this.#expandedMembers = new Map(
      [...this.#expandedMembers].filter(([identity]) => visible.has(identity)),
    );
  }

  #timeRange(): TimeRange {
    switch (this.range) {
      case 'today':
      case 'yesterday':
      case 'since_last_restart':
        return { kind: this.range };
      case 'last7_days':
        return { kind: 'last7_days' };
      case 'custom':
        return { kind: 'custom', from: this.customFrom, to: this.customTo };
    }
  }

  #syncHash(win: Window | undefined = globalThis.window): void {
    if (!win) return;
    const query = win.location.hash.includes('?') ? win.location.hash.slice(win.location.hash.indexOf('?') + 1) : '';
    const params = new URLSearchParams(query);
    params.set('range', this.range);
    params.set('filter', this.filter);
    if (this.entityId) params.set('entity', this.entityId);
    else params.delete('entity');
    if (this.range === 'custom') {
      params.set('from', String(this.customFrom));
      params.set('to', String(this.customTo));
    } else {
      params.delete('from');
      params.delete('to');
    }
    const next = `#/timeline?${params.toString()}`;
    win.history.replaceState(null, '', next);
    // replaceState does not emit hashchange, but the app router must still
    // leave the current screen when search deep-links to an entity.
    win.dispatchEvent(new HashChangeEvent('hashchange'));
  }
}

function validTimestamp(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 0;
}

export const timeline = new TimelineStore();
