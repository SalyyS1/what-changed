/**
 * Collector health for Settings controls and global health indicators.
 *
 * Rust responses remain authoritative; event notices only trigger reconciled
 * reads so the window never invents collector state.
 */

import { failed, loading, ready, type AsyncState } from '../types/async-state';
import type { CollectorStatusDto } from '../tauri/bindings';
import { getCollectorStatus, setCollectorEnabled, toAppError } from '../tauri/client';
import { onCollectorStatusChanged, type Unlisten } from '../tauri/events';

interface CollectorDependencies {
  getStatus: () => Promise<CollectorStatusDto[]>;
  setEnabled: (collectorId: string, enabled: boolean) => Promise<CollectorStatusDto[]>;
  subscribe: (handler: () => void) => Promise<Unlisten>;
}

export function isCollectorDegraded(collector: CollectorStatusDto): boolean {
  return collector.enabled && (collector.state === 'failed' || collector.state === 'degraded' || collector.state === 'unavailable');
}

function spokenState(state: CollectorStatusDto['state']): string {
  if (state === 'idle') return 'running';
  return state;
}

function counted(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? '' : 's'}`;
}

function statusAnnouncement(before: readonly CollectorStatusDto[], after: readonly CollectorStatusDto[]): string {
  const previous = new Map(before.map((row) => [row.id, row]));
  const messages: string[] = [];
  for (const row of after) {
    const old = previous.get(row.id);
    if (!old) continue;
    const name = row.descriptor.display_name;
    const previousState = spokenState(old.state);
    const nextState = spokenState(row.state);
    if (previousState !== nextState) {
      messages.push(`${name} is now ${nextState}.`);
    }
    const queueDrops = row.queue_drops - old.queue_drops;
    if (queueDrops > 0) messages.push(`${name} reported ${counted(queueDrops, 'new queue drop')}.`);
    const lostRows = row.failed_commit_rows - old.failed_commit_rows;
    if (lostRows > 0) messages.push(`${name} reported ${counted(lostRows, 'row')} lost after a failed database commit.`);
    const rejectedCycles = row.cycle_admission_drops - old.cycle_admission_drops;
    if (rejectedCycles > 0) messages.push(`${name} reported ${counted(rejectedCycles, 'atomic cycle')} rejected before database admission.`);
    const failedCycles = row.failed_cycles - old.failed_cycles;
    if (failedCycles > 0) messages.push(`${name} reported ${counted(failedCycles, 'admitted atomic cycle')} failed.`);
  }
  return messages.join(' ');
}

export class CollectorsStore {
  state = $state<AsyncState<CollectorStatusDto[]>>(loading());
  pending = $state<Set<string>>(new Set());
  errors = $state<Record<string, string>>({});
  announcement = $state('');
  #deps: CollectorDependencies;
  #request = 0;
  #refreshQueued = false;
  #refreshCycle: Promise<void> | null = null;

  constructor(dependencies: CollectorDependencies = {
    getStatus: getCollectorStatus,
    setEnabled: setCollectorEnabled,
    subscribe: (handler) => onCollectorStatusChanged(handler),
  }) {
    this.#deps = dependencies;
  }

  get rows(): CollectorStatusDto[] {
    return this.state.status === 'ready' ? this.state.data : [];
  }

  async load(): Promise<void> {
    this.#refreshQueued = true;
    if (this.pending.size > 0) {
      return;
    }
    if (this.#refreshCycle !== null) return this.#refreshCycle;

    // Defer starting the drain one microtask so the promise is installed
    // before a dependency can synchronously trigger another refresh callback.
    const cycle = Promise.resolve().then(() => this.#drainRefreshes());
    this.#refreshCycle = cycle;
    try {
      await cycle;
    } finally {
      if (this.#refreshCycle === cycle) {
        this.#refreshCycle = null;
        // A callback can queue work after the drain resolves but before this
        // continuation runs. Re-check here so that edge cannot strand it.
        if (this.#refreshQueued && this.pending.size === 0) void this.load();
      }
    }
  }

  async #drainRefreshes(): Promise<void> {
    while (this.#refreshQueued && this.pending.size === 0) {
      this.#refreshQueued = false;
      await this.#loadOnce();
    }
  }

  async #loadOnce(): Promise<void> {
    const request = ++this.#request;
    const hadRows = this.state.status === 'ready';
    const loadingDelay = setTimeout(() => {
      if (request === this.#request && !hadRows) this.state = loading();
    }, 300);
    try {
      const rows = await this.#deps.getStatus();
      if (request === this.#request) {
        const before = this.rows;
        this.state = ready(rows);
        if (hadRows) {
          const message = statusAnnouncement(before, rows);
          if (message) this.announcement = message;
        }
      }
    } catch (reason) {
      if (request !== this.#request) return;
      const error = toAppError(reason);
      if (!hadRows) this.state = failed(error.message, error.retryable ? () => void this.load() : undefined);
    } finally {
      clearTimeout(loadingDelay);
    }
  }

  /** Subscribe to health changes. Returns the teardown. */
  async attach(): Promise<Unlisten> {
    let unlisten: Unlisten = () => {};
    try {
      unlisten = await this.#deps.subscribe(() => void this.load());
    } catch {
      // Event delivery is an optimization. The authoritative initial read and
      // fallback poll below must still start when WebView event setup fails.
    }
    await this.load();
    // Events are invalidation hints. A quiet fallback catches missed delivery
    // and autonomous telemetry changes without spinning or duplicating workers.
    const poll = setInterval(() => void this.load(), 10_000);
    let detached = false;
    return () => {
      if (detached) return;
      detached = true;
      clearInterval(poll);
      try {
        unlisten();
      } catch {
        // The WebView may already be gone. Teardown remains safe and local.
      }
    };
  }

  /** Switch one collector on or off. The backend returns the whole list. */
  async setEnabled(collectorId: string, enabled: boolean): Promise<boolean> {
    const row = this.rows.find((candidate) => candidate.id === collectorId);
    if (!row) return false;
    const blocked = row.state === 'unavailable' || row.requires_admin || row.descriptor.requires_admin;
    if (blocked || this.pending.size > 0) {
      return false;
    }
    const request = ++this.#request;
    this.pending = new Set(this.pending).add(collectorId);
    const nextErrors = { ...this.errors };
    delete nextErrors[collectorId];
    this.errors = nextErrors;
    try {
      const rows = await this.#deps.setEnabled(collectorId, enabled);
      if (request === this.#request) this.state = ready(rows);
      return true;
    } catch (reason) {
      const error = toAppError(reason);
      this.errors = { ...this.errors, [collectorId]: error.message };
      if (this.state.status !== 'ready') {
        this.state = failed(error.message);
      }
      return false;
    } finally {
      const next = new Set(this.pending);
      next.delete(collectorId);
      this.pending = next;
      if (this.#refreshQueued) {
        void this.load();
      }
    }
  }
}

export const collectors = new CollectorsStore();
