/**
 * Live Diagnostics snapshot.
 *
 * Collector events are only invalidation notices. The command response stays
 * authoritative, and refreshes are coalesced so a status storm costs at most
 * one in-flight read plus one trailing reconciliation.
 */

import type { DiagnosticsDto } from '../tauri/bindings';
import { backendAvailable, getDiagnostics, toAppError } from '../tauri/client';
import { onCollectorStatusChanged, type Unlisten } from '../tauri/events';
import { failed, loading, ready, type AsyncState } from '../types/async-state';

const RECONCILE_INTERVAL_MS = 10_000;
const NOOP: Unlisten = () => {};

interface DiagnosticsDependencies {
  getSnapshot: () => Promise<DiagnosticsDto>;
  subscribe: (handler: () => void) => Promise<Unlisten>;
  canReconcile: () => boolean;
}

export class DiagnosticsStore {
  state = $state<AsyncState<DiagnosticsDto>>(loading());
  /** A poll failed after a usable snapshot loaded, so the visible data may be stale. */
  refreshError = $state<string | null>(null);
  #deps: DiagnosticsDependencies;
  #refreshQueued = false;
  #refreshCycle: Promise<void> | null = null;

  constructor(dependencies: DiagnosticsDependencies = {
    getSnapshot: getDiagnostics,
    subscribe: (handler) => onCollectorStatusChanged(handler),
    canReconcile: backendAvailable,
  }) {
    this.#deps = dependencies;
  }

  /** Read the authoritative snapshot. This promise resolves even on an IPC error. */
  async refresh(): Promise<void> {
    this.#refreshQueued = true;
    if (this.#refreshCycle !== null) return this.#refreshCycle;

    // Install the shared promise before a synchronously delivered event can
    // request another refresh. Any such event becomes one trailing read.
    const cycle = Promise.resolve().then(() => this.#drainRefreshes());
    this.#refreshCycle = cycle;
    try {
      await cycle;
    } finally {
      if (this.#refreshCycle === cycle) {
        this.#refreshCycle = null;
        if (this.#refreshQueued) void this.refresh();
      }
    }
  }

  async #drainRefreshes(): Promise<void> {
    while (this.#refreshQueued) {
      this.#refreshQueued = false;
      await this.#refreshOnce();
    }
  }

  async #refreshOnce(): Promise<void> {
    const hadSnapshot = this.state.status === 'ready';
    if (!hadSnapshot) this.state = loading();
    try {
      const snapshot = await this.#deps.getSnapshot();
      this.state = ready(snapshot);
      this.refreshError = null;
    } catch (reason) {
      const error = toAppError(reason);
      if (hadSnapshot) {
        // Keep useful diagnostics visible, but never imply that stale values
        // are still live when reconciliation failed.
        this.refreshError = error.message;
      } else {
        this.state = failed(
          error.message,
          error.retryable ? () => void this.refresh() : undefined,
        );
      }
    }
  }

  /** Subscribe before the initial read, then reconcile missed notices at a bounded rate. */
  async attach(): Promise<Unlisten> {
    let detached = false;
    let unlisten = NOOP;
    try {
      const subscribed = await this.#deps.subscribe(() => {
        if (!detached) void this.refresh();
      });
      if (detached) subscribed();
      else unlisten = subscribed;
    } catch {
      // Event delivery is an optimization. The authoritative read and bounded
      // fallback below still make Diagnostics useful.
    }

    // Teardown must be available even while the initial IPC read is pending.
    if (!detached) void this.refresh();
    const poll = !detached && this.#deps.canReconcile()
      ? setInterval(() => void this.refresh(), RECONCILE_INTERVAL_MS)
      : null;

    return () => {
      if (detached) return;
      detached = true;
      if (poll !== null) clearInterval(poll);
      try {
        unlisten();
      } catch {
        // The WebView may already be gone. Teardown remains local and safe.
      }
    };
  }
}

export const diagnostics = new DiagnosticsStore();
