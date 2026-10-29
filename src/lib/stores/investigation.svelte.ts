/** Investigation draft, command lifecycle, history and selected result. */

import { toInvestigationView } from '../../features/investigation/investigation-adapter';
import type {
  InvestigationView,
  InvestigationWindow,
} from '../../features/investigation/investigation-types';
import * as client from '../tauri/client';
import type { InvestigationWindow as BackendInvestigationWindow } from '../tauri/bindings';
import { onAnalysisCompleted } from '../tauri/events';
import { projects } from './projects.svelte';
import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';

const WINDOW_KEY = 'what-changed:investigation-window';

type UntrustedResult = unknown;
type Unlisten = () => void;

interface InvestigationDependencies {
  list: (limit: number) => Promise<UntrustedResult[]>;
  start: (problem: string, window: InvestigationWindow, projectId: string | null) => Promise<UntrustedResult>;
  get: (id: number) => Promise<UntrustedResult>;
  remove: (id: number) => Promise<void>;
  subscribe: (handler: (id: number) => void) => Promise<Unlisten>;
}

function backendWindow(window: InvestigationWindow): BackendInvestigationWindow {
  if (window.kind === 'since_checkpoint') {
    return { kind: window.kind, checkpoint_id: window.id };
  }
  if (window.kind === 'custom') {
    return { kind: window.kind, from_ms: window.from, to_ms: window.to };
  }
  return window;
}

const dependencies: InvestigationDependencies = {
  list: (limit) => client.listInvestigations(limit) as Promise<unknown[]>,
  start: (problem, window, projectId) => client.startInvestigation({ problem, window: backendWindow(window), project_id: projectId }),
  get: (id) => client.getInvestigation(id),
  remove: (id) => client.deleteInvestigation(id),
  subscribe: onAnalysisCompleted,
};

function savedWindow(): InvestigationWindow {
  try {
    const value = globalThis.localStorage?.getItem(WINDOW_KEY);
    if (!value) return { kind: 'today' };
    const parsed = JSON.parse(value) as Partial<InvestigationWindow>;
    if (parsed.kind === 'last_hour' || parsed.kind === 'today' || parsed.kind === 'since_yesterday' || parsed.kind === 'since_last_restart') return { kind: parsed.kind };
    if (parsed.kind === 'since_checkpoint' && typeof parsed.id === 'number' && parsed.id > 0) return { kind: parsed.kind, id: parsed.id };
    if (parsed.kind === 'custom' && typeof parsed.from === 'number' && typeof parsed.to === 'number' && parsed.from <= parsed.to) {
      return { kind: parsed.kind, from: parsed.from, to: parsed.to };
    }
  } catch { /* Invalid local draft is safely ignored. */ }
  return { kind: 'today' };
}

export class InvestigationStore {
  state = $state<AsyncState<InvestigationView[]>>(loading());
  selected = $state<InvestigationView | null>(null);
  running = $state(false);
  deleting = $state<Set<number>>(new Set());
  error = $state<string | null>(null);
  // Problem text may contain pasted credentials. Keep it in memory until the
  // sanitized backend persists the completed investigation.
  draft = $state('');
  window = $state<InvestigationWindow>(savedWindow());
  projectId = $state<string | null>(null);
  #deps: InvestigationDependencies;
  #startPromise: Promise<InvestigationView | null> | null = null;
  #generation = 0;

  constructor(overrides: InvestigationDependencies = dependencies) { this.#deps = overrides; }
  get rows(): InvestigationView[] { return this.state.status === 'ready' ? this.state.data : []; }

  setDraft(value: string): void {
    this.draft = value;
  }

  setWindow(value: InvestigationWindow): void {
    this.window = value;
    globalThis.localStorage?.setItem(WINDOW_KEY, JSON.stringify(value));
  }

  setProjectId(projectId: string | null): void {
    this.projectId = projectId;
  }

  async load(): Promise<boolean> {
    const generation = ++this.#generation;
    this.state = loading();
    try {
      const rows = (await this.#deps.list(50)).map(toInvestigationView);
      if (generation !== this.#generation) return false;
      this.state = rows.length === 0 ? empty() : ready(rows);
      if (this.selected === null && rows.length > 0) this.selected = rows[0] ?? null;
      this.error = null;
      return true;
    } catch (reason) {
      if (generation !== this.#generation) return false;
      const error = client.toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => void this.load() : undefined);
      return false;
    }
  }

  start(): Promise<InvestigationView | null> {
    if (this.#startPromise !== null) return this.#startPromise;
    this.#startPromise = this.#start();
    return this.#startPromise;
  }

  async #start(): Promise<InvestigationView | null> {
    const problem = this.draft.trim();
    if (!problem) return null;
    const generation = ++this.#generation;
    this.running = true;
    this.error = null;
    try {
      const result = toInvestigationView(await this.#deps.start(problem, this.window, this.projectId));
      if (generation === this.#generation) {
        this.selected = result;
        const rows = [result, ...this.rows.filter((row) => row.id !== result.id)];
        this.state = ready(rows);
      }
      return result;
    } catch (reason) {
      if (generation === this.#generation) this.error = client.toAppError(reason).message;
      return null;
    } finally {
      this.running = false;
      this.#startPromise = null;
    }
  }

  async select(id: number): Promise<boolean> {
    const existing = this.rows.find((row) => row.id === id);
    const generation = ++this.#generation;
    if (existing) { this.selected = existing; return true; }
    try {
      const selected = toInvestigationView(await this.#deps.get(id));
      if (generation !== this.#generation) return false;
      this.selected = selected;
      return true;
    } catch (reason) {
      if (generation === this.#generation) this.error = client.toAppError(reason).message;
      return false;
    }
  }

  async remove(id: number): Promise<boolean> {
    if (this.deleting.has(id)) return false;
    this.deleting = new Set(this.deleting).add(id);
    const generation = ++this.#generation;
    try {
      await this.#deps.remove(id);
      if (generation !== this.#generation) return false;
      const rows = this.rows.filter((row) => row.id !== id);
      this.state = rows.length > 0 ? ready(rows) : empty();
      if (this.selected?.id === id) this.selected = rows[0] ?? null;
      return true;
    } catch (reason) {
      if (generation === this.#generation) this.error = client.toAppError(reason).message;
      return false;
    } finally {
      const next = new Set(this.deleting); next.delete(id); this.deleting = next;
    }
  }

  async attach(): Promise<Unlisten> {
    return this.#deps.subscribe((id) => {
      const generation = ++this.#generation;
      void this.#deps.get(id).then((raw) => {
        if (generation !== this.#generation) return;
        const result = toInvestigationView(raw);
        this.state = ready([result, ...this.rows.filter((row) => row.id !== result.id)]);
        this.selected = result;
      }).catch(() => {
        if (generation === this.#generation) void this.load();
      });
    }).catch(() => () => {});
  }
}

export const investigation = new InvestigationStore();
projects.onSelectionChange((projectId) => investigation.setProjectId(projectId));
