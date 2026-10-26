/** Checkpoint list, mutations, comparison, and live capture progress. */

import type { CheckpointDiffDto, CheckpointDto } from '../tauri/bindings';
import {
  compareCheckpoint,
  createCheckpoint,
  deleteCheckpoint,
  listCheckpoints,
  listenCheckpointProgress,
  renameCheckpoint,
  toAppError,
  type CheckpointProgressNotice,
  type CheckpointProgressState,
} from '../tauri/client';
import { onCheckpointSaved } from '../tauri/events';
import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';

export interface CheckpointProgressItem { collectorId: string; label: string; state: CheckpointProgressState; }
type Unlisten = () => void;

interface CheckpointDependencies {
  list: () => Promise<CheckpointDto[]>;
  create: (name?: string, reason?: string) => Promise<CheckpointDto>;
  rename: (id: number, name: string) => Promise<CheckpointDto>;
  remove: (id: number) => Promise<void>;
  compare: (id: number, against?: number) => Promise<CheckpointDiffDto>;
  subscribe: (handler: (notice: CheckpointProgressNotice) => void) => Promise<Unlisten>;
  subscribeSaved: (handler: () => void) => Promise<Unlisten>;
}

const dependencies: CheckpointDependencies = {
  list: listCheckpoints,
  create: createCheckpoint,
  rename: renameCheckpoint,
  remove: deleteCheckpoint,
  compare: compareCheckpoint,
  subscribe: listenCheckpointProgress,
  subscribeSaved: (handler) => onCheckpointSaved(handler),
};

export class CheckpointsStore {
  state = $state<AsyncState<CheckpointDto[]>>(loading());
  creating = $state(false);
  mutating = $state<Set<number>>(new Set());
  comparing = $state(false);
  comparison = $state<CheckpointDiffDto | null>(null);
  selectedId = $state<number | null>(null);
  error = $state<string | null>(null);
  progress = $state<CheckpointProgressItem[]>([]);
  progressCompleted = $state(0);
  progressTotal = $state(0);
  #deps: CheckpointDependencies;
  #createPromise: Promise<CheckpointDto | null> | null = null;
  #request = 0;

  constructor(overrides: CheckpointDependencies = dependencies) { this.#deps = overrides; }
  get rows(): CheckpointDto[] { return this.state.status === 'ready' ? this.state.data : []; }

  async load(preserveExisting = false): Promise<boolean> {
    const request = ++this.#request;
    if (!preserveExisting) this.state = loading();
    try {
      const rows = await this.#deps.list();
      if (request === this.#request) {
        this.state = rows.length === 0 ? empty() : ready(rows);
        this.error = null;
      }
      return true;
    } catch (reason) {
      if (request !== this.#request) return true;
      const error = toAppError(reason);
      if (preserveExisting && (this.state.status === 'ready' || this.state.status === 'empty')) this.error = error.message;
      else this.state = failed(error.message, error.retryable ? () => void this.load() : undefined);
      return false;
    }
  }

  /** Duplicate clicks share one in-flight command instead of starting another sweep. */
  create(name: string, reason: string): Promise<CheckpointDto | null> {
    if (this.#createPromise !== null) return this.#createPromise;
    this.#createPromise = this.#create(name, reason);
    return this.#createPromise;
  }

  async #create(name: string, reason: string): Promise<CheckpointDto | null> {
    this.creating = true;
    this.error = null;
    this.progress = [];
    this.progressCompleted = 0;
    this.progressTotal = 0;
    try {
      const checkpoint = await this.#deps.create(name.trim() || undefined, reason.trim() || undefined);
      await this.load(true);
      return checkpoint;
    } catch (reason) {
      this.error = toAppError(reason).message;
      return null;
    } finally {
      this.creating = false;
      this.progress = this.progress.filter((item) => item.collectorId !== 'capture');
      this.#createPromise = null;
    }
  }

  async rename(id: number, name: string): Promise<boolean> {
    const trimmed = name.trim();
    if (trimmed.length === 0 || this.mutating.has(id)) return false;
    return this.#mutate(id, () => this.#deps.rename(id, trimmed));
  }

  async remove(id: number): Promise<boolean> {
    if (this.mutating.has(id)) return false;
    return this.#mutate(id, () => this.#deps.remove(id));
  }

  async #mutate(id: number, operation: () => Promise<unknown>): Promise<boolean> {
    this.mutating = new Set(this.mutating).add(id);
    this.error = null;
    try {
      await operation();
      if (this.selectedId === id) this.closeComparison();
      return await this.load(true);
    } catch (reason) {
      this.error = toAppError(reason).message;
      return false;
    } finally {
      const next = new Set(this.mutating); next.delete(id); this.mutating = next;
    }
  }

  async compareToNow(id: number): Promise<boolean> {
    if (this.comparing) return false;
    this.comparing = true; this.selectedId = id; this.comparison = null; this.error = null;
    try {
      this.comparison = await this.#deps.compare(id);
      return true;
    } catch (reason) {
      this.error = toAppError(reason).message; this.selectedId = null; return false;
    } finally { this.comparing = false; }
  }

  closeComparison(): void { this.selectedId = null; this.comparison = null; this.comparing = false; }

  async attachProgress(): Promise<Unlisten> {
    const unlisteners = await Promise.all([
      this.#deps.subscribe((notice) => this.#applyProgress(notice)).catch(() => () => {}),
      this.#deps.subscribeSaved(() => { void this.load(true); }).catch(() => () => {}),
    ]);
    return () => { for (const unlisten of unlisteners) unlisten(); };
  }

  #applyProgress(notice: CheckpointProgressNotice): void {
    if (!this.creating) return;
    this.progressCompleted = Math.max(0, notice.completed);
    this.progressTotal = Math.max(this.progressCompleted, notice.total);
    const item: CheckpointProgressItem = {
      collectorId: notice.collector_id,
      label: notice.collector_name || humanizeCollector(notice.collector_id),
      state: notice.status,
    };
    const index = this.progress.findIndex((entry) => entry.collectorId === item.collectorId);
    this.progress = index < 0 ? [...this.progress, item] : this.progress.map((entry, i) => i === index ? item : entry);
  }
}

function humanizeCollector(id: string): string {
  return id.replace(/[-_]/g, ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export const checkpoints = new CheckpointsStore();
