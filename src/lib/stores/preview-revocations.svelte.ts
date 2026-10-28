import type { PreviewRunnerStatusDto } from '../tauri/bindings';
import { cancelPreviewRun, getPreviewRunnerStatus, toAppError } from '../tauri/client';

export const terminalPreviewState = (state: string): boolean => ['completed', 'cancelled', 'interrupted', 'unavailable', 'failed'].includes(state);
export interface PreviewRevocationDependencies {
  cancel: typeof cancelPreviewRun;
  status: typeof getPreviewRunnerStatus;
}
export interface PreviewRevocation {
  app_id: string;
  run_id: string;
  busy: boolean;
  error: string | null;
}
interface PendingRevocation {
  dependencies: PreviewRevocationDependencies;
  promise: Promise<PreviewRunnerStatusDto | null> | null;
  timer: ReturnType<typeof setTimeout> | null;
  attempts: number;
}
const defaults = { cancel: cancelPreviewRun, status: getPreviewRunnerStatus };
const identity = (appId: string, runId: string): string => JSON.stringify([appId, runId]);

/** Retains only existing sessions awaiting native revocation confirmation. */
export class PreviewRevocationsStore {
  sessions = $state<PreviewRevocation[]>([]);
  #pending = new Map<string, PendingRevocation>();
  #attachments = 0;
  #epoch = 0;

  /** The root app owns retry timers; navigation does not detach this owner. */
  attach(): () => void {
    ++this.#attachments;
    for (const session of this.sessions) void this.retry(session.app_id, session.run_id);
    let detached = false;
    return () => {
      if (detached) return;
      detached = true;
      if (--this.#attachments !== 0) return;
      ++this.#epoch;
      for (const pending of this.#pending.values()) {
        if (pending.timer !== null) clearTimeout(pending.timer);
        pending.timer = null;
      }
    };
  }

  revoke(session: PreviewRunnerStatusDto, dependencies: PreviewRevocationDependencies = defaults): Promise<PreviewRunnerStatusDto | null> {
    if (terminalPreviewState(session.lifecycle)) return Promise.resolve(session);
    const key = identity(session.app_id, session.run_id);
    if (!this.#pending.has(key)) {
      this.#pending.set(key, { dependencies, promise: null, timer: null, attempts: 0 });
      this.sessions = [...this.sessions, { app_id: session.app_id, run_id: session.run_id, busy: false, error: null }];
    }
    return this.retry(session.app_id, session.run_id);
  }

  retry(appId: string, runId: string): Promise<PreviewRunnerStatusDto | null> {
    const key = identity(appId, runId);
    const pending = this.#pending.get(key);
    if (!pending) return Promise.resolve(null);
    if (pending.promise) return pending.promise;
    if (pending.timer !== null) clearTimeout(pending.timer);
    pending.timer = null;
    this.#update(key, { busy: true, error: null });
    pending.promise = this.#attempt(appId, runId, pending, this.#epoch).finally(() => {
      pending.promise = null;
      if (this.#pending.get(key) !== pending) return;
      this.#update(key, { busy: false });
      if (this.#attachments === 0) return;
      // One request at a time, with backoff capped at ten seconds.
      const delay = Math.min(1_000 * 2 ** Math.min(pending.attempts++, 4), 10_000);
      pending.timer = setTimeout(() => {
        pending.timer = null;
        void this.retry(appId, runId);
      }, delay);
    });
    return pending.promise;
  }

  async #attempt(appId: string, runId: string, pending: PendingRevocation, epoch: number): Promise<PreviewRunnerStatusDto | null> {
    const key = identity(appId, runId);
    let cancellationError: string | null = null;
    try { await pending.dependencies.cancel(appId, runId); }
    catch (reason) { cancellationError = toAppError(reason).message; }
    // A closing app leaves process cleanup to native lifecycle ownership.
    if (epoch !== this.#epoch) return null;
    try {
      const next = await pending.dependencies.status(appId, runId);
      if (next.app_id !== appId || next.run_id !== runId) {
        this.#update(key, { error: 'Native status did not match this preview session. Revocation is not confirmed.' });
        return null;
      }
      if (terminalPreviewState(next.lifecycle)) {
        this.#pending.delete(key);
        this.sessions = this.sessions.filter((session) => identity(session.app_id, session.run_id) !== key);
      } else {
        this.#update(key, { error: cancellationError ?? 'The test browser has not confirmed that it stopped.' });
      }
      return next;
    } catch (reason) {
      const statusError = toAppError(reason).message;
      this.#update(key, { error: cancellationError ? `${cancellationError} Status: ${statusError}` : statusError });
      return null;
    }
  }

  #update(key: string, patch: Partial<Pick<PreviewRevocation, 'busy' | 'error'>>): void {
    this.sessions = this.sessions.map((session) => identity(session.app_id, session.run_id) === key ? { ...session, ...patch } : session);
  }
}

export const previewRevocations = new PreviewRevocationsStore();
