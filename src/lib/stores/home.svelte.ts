import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import type { AppStatusDto, CollectorStatusDto, DashboardSummaryDto, TimeRange } from '../tauri/bindings';
import { getAppStatus, getCollectorStatus, getDashboardSummary, toAppError } from '../tauri/client';
import { isCollectorDegraded } from './collectors.svelte';

export type GlobalHealth = 'active' | 'paused' | 'degraded' | 'recovery';

export interface HomeSnapshot {
  summary: DashboardSummaryDto;
  appStatus: AppStatusDto;
  collectors: CollectorStatusDto[];
  health: GlobalHealth;
  startedAt: number;
}

interface HomeDependencies {
  summary: typeof getDashboardSummary;
  status: typeof getAppStatus;
  collectors: typeof getCollectorStatus;
}

export function deriveGlobalHealth(
  status: AppStatusDto,
  collectors: CollectorStatusDto[],
): GlobalHealth {
  if (status.db.state !== 'ready') return 'recovery';
  if (collectors.some(isCollectorDegraded)) {
    return 'degraded';
  }
  return status.monitoring.paused ? 'paused' : 'active';
}

export class HomeStore {
  state = $state<AsyncState<HomeSnapshot>>(loading());
  range = $state<TimeRange>({ kind: 'today' });
  #startedAt = Date.now();
  #request = 0;
  #deps: HomeDependencies;

  constructor(dependencies: HomeDependencies = {
    summary: getDashboardSummary,
    status: getAppStatus,
    collectors: getCollectorStatus,
  }) {
    this.#deps = dependencies;
  }

  async load(): Promise<void> {
    const request = ++this.#request;
    const loadingDelay = setTimeout(() => {
      if (request === this.#request) this.state = loading();
    }, 300);
    try {
      const [summary, appStatus, collectors] = await Promise.all([
        this.#deps.summary(this.range),
        this.#deps.status(),
        this.#deps.collectors(),
      ]);
      if (request !== this.#request) return;
      if (summary.today.total === 0 && appStatus.db.state === 'ready') {
        this.state = empty(`What Changed started recording at ${new Intl.DateTimeFormat('en-US', {
          hour: 'numeric',
          minute: '2-digit',
        }).format(this.#startedAt)}.`);
        return;
      }
      this.state = ready({
        summary,
        appStatus,
        collectors,
        health: deriveGlobalHealth(appStatus, collectors),
        startedAt: this.#startedAt,
      });
    } catch (reason) {
      if (request !== this.#request) return;
      const error = toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => void this.load() : undefined);
    } finally {
      clearTimeout(loadingDelay);
    }
  }

  async setRange(range: TimeRange): Promise<void> {
    this.range = range;
    await this.load();
  }
}

export const home = new HomeStore();
