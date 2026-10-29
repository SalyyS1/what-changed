/**
 * Whether the recorder is recording, and the app's boot status.
 *
 * The value is authoritative in Rust — the tray can pause monitoring without
 * the window being open — so this store subscribes to the backend event rather
 * than tracking its own toggles. A local flip would be a second source of
 * truth that is wrong every time the tray is used.
 */

import type { AppStatusDto, MonitoringDto } from '../tauri/bindings';
import { backendAvailable, getAppStatus, setMonitoringPaused, toAppError } from '../tauri/client';
import { onMonitoringStateChanged, type Unlisten } from '../tauri/events';

const RECONCILE_INTERVAL_MS = 10_000;
const NOOP: Unlisten = () => {};

interface MonitoringDependencies {
  getStatus: () => Promise<AppStatusDto>;
  setPaused: (paused: boolean, minutes?: number) => Promise<MonitoringDto>;
  subscribe: (handler: (state: MonitoringDto) => void) => Promise<Unlisten>;
  backendAvailable: () => boolean;
}

export class MonitoringStore {
  paused = $state(false);
  /** False until the initial backend status request has settled. */
  resolved = $state(false);
  /** When a timed pause lifts, epoch milliseconds. `null` if indefinite. */
  resumeAt = $state<number | null>(null);
  /** The last boot status, `null` until it has been read. */
  status = $state<AppStatusDto | null>(null);
  /** A real Tauri bridge answered neither status nor error details. */
  recoveryRequired = $state(false);
  pending = $state(false);
  error = $state<string | null>(null);
  #deps: MonitoringDependencies;
  #stateRevision = 0;
  #statusRequest = 0;
  #appliedStatusRequest = 0;

  constructor(dependencies: MonitoringDependencies = {
    getStatus: getAppStatus,
    setPaused: setMonitoringPaused,
    subscribe: onMonitoringStateChanged,
    backendAvailable,
  }) {
    this.#deps = dependencies;
  }

  /** What the titlebar pill shows. */
  get label(): 'Active' | 'Paused' | 'Unknown' {
    if (this.recoveryRequired) return 'Unknown';
    return this.paused ? 'Paused' : 'Active';
  }

  /** Whether ordinary screens must be hidden behind the recovery surface. */
  get needsRecovery(): boolean {
    return (
      this.recoveryRequired ||
      (this.status?.db.state !== undefined && this.status.db.state !== 'ready')
    );
  }

  /**
   * True when closing the window would exit rather than hide, because the tray
   * icon could not be created. The shell says so rather than letting a user
   * lose the window to a tray that is not there.
   */
  get closeWouldExit(): boolean {
    return this.status !== null && !this.status.tray_available;
  }

  apply(state: MonitoringDto): void {
    this.#stateRevision += 1;
    this.#applyState(state);
    if (this.status !== null) {
      this.status = { ...this.status, monitoring: state };
    }
  }

  #applyState(state: MonitoringDto): void {
    this.paused = state.paused;
    this.resumeAt = state.resume_at;
    this.error = null;
  }

  #currentState(): MonitoringDto {
    return { paused: this.paused, resume_at: this.resumeAt };
  }

  /** Read authoritative app status; a real bridge failure fails closed into recovery. */
  async refresh(): Promise<void> {
    const request = ++this.#statusRequest;
    const stateRevision = this.#stateRevision;
    try {
      const status = await this.#deps.getStatus();
      if (request < this.#appliedStatusRequest) return;
      this.#appliedStatusRequest = request;
      this.recoveryRequired = false;
      if (stateRevision === this.#stateRevision) {
        this.#applyState(status.monitoring);
        this.status = status;
      } else {
        // An event or command response landed after this snapshot began. Keep
        // the newer monitoring state while still accepting the boot metadata.
        this.status = { ...status, monitoring: this.#currentState() };
      }
    } catch {
      if (request < this.#appliedStatusRequest) return;
      this.#appliedStatusRequest = request;
      this.status = null;
      // A browser preview has no recorder by design. Inside a real Tauri
      // window, however, a missing status is never evidence of healthy
      // monitoring; fail closed into recovery instead of claiming Active.
      this.recoveryRequired = this.#deps.backendAvailable();
    } finally {
      this.resolved = true;
    }
  }

  /** Subscribe before the first snapshot, then reconcile missed events. */
  async attach(): Promise<Unlisten> {
    let unlisten = NOOP;
    try {
      unlisten = await this.#deps.subscribe((state) => this.apply(state));
    } catch {
      // A failed event channel must not reject App's mount. The bounded poll
      // below remains authoritative and catches tray/timed-watchdog changes.
    }
    await this.refresh();
    let reconciling = false;
    const poll = this.#deps.backendAvailable()
      ? setInterval(() => {
          if (reconciling) return;
          reconciling = true;
          void this.refresh().finally(() => {
            reconciling = false;
          });
        }, RECONCILE_INTERVAL_MS)
      : null;
    let detached = false;
    return () => {
      if (detached) return;
      detached = true;
      if (poll !== null) clearInterval(poll);
      try {
        unlisten();
      } catch {
        // Teardown is best effort; unmount must remain safe if WebView closed.
      }
    };
  }

  async setPaused(paused: boolean, minutes?: number): Promise<boolean> {
    if (this.pending) return false;
    this.pending = true;
    this.error = null;
    const stateRevision = this.#stateRevision;
    try {
      const state = await this.#deps.setPaused(paused, minutes);
      if (stateRevision === this.#stateRevision) this.apply(state);
      return true;
    } catch (reason) {
      this.error = toAppError(reason).message;
      // The backend refused; its own state is unchanged, and the next event
      // or refresh will restate it.
      return false;
    } finally {
      this.pending = false;
    }
  }
}

export const monitoring = new MonitoringStore();
