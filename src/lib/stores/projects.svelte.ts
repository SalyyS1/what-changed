/**
 * Explicit project roots used as recovery workspaces. The backend owns path
 * canonicalization and the baseline checkpoint; this store only manages UI
 * lifecycle and refreshes from authoritative responses.
 */

import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import type { ProjectDto } from '../tauri/bindings';
import {
  addProject,
  clearProjectBaseline,
  listProjects,
  markProjectWorking,
  removeProject,
  toAppError,
} from '../tauri/client';
import { pickDirectory } from '../tauri/dialog';

export const PROJECT_LIMIT = 20;
const SELECTED_PROJECT_KEY = 'what-changed:selected-project';
export type Project = ProjectDto;

interface ProjectDependencies {
  list: () => Promise<Project[]>;
  add: (path: string) => Promise<Project>;
  remove: (projectId: string) => Promise<void>;
  markWorking: (projectId: string) => Promise<Project>;
  clearBaseline: (projectId: string) => Promise<Project>;
  chooseDirectory: () => Promise<string | null>;
}

const dependencies: ProjectDependencies = {
  list: listProjects,
  add: addProject,
  remove: removeProject,
  markWorking: markProjectWorking,
  clearBaseline: clearProjectBaseline,
  chooseDirectory: () => pickDirectory('Choose a project root'),
};

export class ProjectsStore {
  state = $state<AsyncState<Project[]>>(loading());
  adding = $state(false);
  marking = $state<Set<string>>(new Set());
  removing = $state<Set<string>>(new Set());
  error = $state<string | null>(null);
  selectedId = $state<string | null>(readSelectedId());
  #selectionListeners = new Set<(projectId: string | null) => void>();
  #deps: ProjectDependencies;
  #request = 0;

  constructor(overrides: ProjectDependencies = dependencies) {
    this.#deps = overrides;
  }

  get rows(): Project[] {
    return this.state.status === 'ready' ? this.state.data : [];
  }

  get loaded(): boolean {
    return this.state.status === 'ready' || this.state.status === 'empty';
  }

  get atLimit(): boolean {
    return this.rows.length >= PROJECT_LIMIT;
  }

  get selected(): Project | null {
    return this.rows.find((row) => row.id === this.selectedId) ?? null;
  }

  select(projectId: string | null): void {
    if (projectId !== null && !this.rows.some((row) => row.id === projectId)) return;
    this.selectedId = projectId;
    if (projectId === null) globalThis.localStorage?.removeItem(SELECTED_PROJECT_KEY);
    else globalThis.localStorage?.setItem(SELECTED_PROJECT_KEY, projectId);
    for (const listener of this.#selectionListeners) listener(projectId);
  }

  onSelectionChange(listener: (projectId: string | null) => void): () => void {
    this.#selectionListeners.add(listener);
    return () => this.#selectionListeners.delete(listener);
  }

  async load(): Promise<void> {
    this.error = null;
    await this.#refresh(this.loaded);
  }

  async chooseAndAdd(): Promise<boolean> {
    if (this.adding || !this.loaded || this.atLimit) return false;
    this.adding = true;
    this.error = null;
    try {
      const path = await this.#deps.chooseDirectory();
      if (path === null) return false;
      await this.#deps.add(path);
      return await this.#refresh(true);
    } catch (reason) {
      this.error = toAppError(reason).message;
      return false;
    } finally {
      this.adding = false;
    }
  }

  async markWorking(projectId: string): Promise<boolean> {
    if (this.marking.has(projectId)) return false;
    this.marking = new Set(this.marking).add(projectId);
    this.error = null;
    try {
      await this.#deps.markWorking(projectId);
      return await this.#refresh(true);
    } catch (reason) {
      this.error = toAppError(reason).message;
      return false;
    } finally {
      const next = new Set(this.marking);
      next.delete(projectId);
      this.marking = next;
    }
  }

  async clearBaseline(projectId: string): Promise<boolean> {
    try {
      await this.#deps.clearBaseline(projectId);
      return await this.#refresh(true);
    } catch (reason) {
      this.error = toAppError(reason).message;
      return false;
    }
  }

  async remove(projectId: string): Promise<boolean> {
    if (this.removing.has(projectId) || !this.rows.some((row) => row.id === projectId)) return false;
    this.removing = new Set(this.removing).add(projectId);
    this.error = null;
    try {
      await this.#deps.remove(projectId);
      return await this.#refresh(true);
    } catch (reason) {
      this.error = toAppError(reason).message;
      return false;
    } finally {
      const next = new Set(this.removing);
      next.delete(projectId);
      this.removing = next;
    }
  }

  async #refresh(preserveExisting: boolean): Promise<boolean> {
    const request = ++this.#request;
    if (!preserveExisting) this.state = loading();
    try {
      const rows = await this.#deps.list();
      if (request === this.#request) {
        this.state = rows.length === 0
          ? empty('Add a project root when you want a focused recovery workspace.')
          : ready(rows);
        if (this.selectedId !== null && !rows.some((row) => row.id === this.selectedId)) this.select(null);
      }
      return true;
    } catch (reason) {
      if (request !== this.#request) return true;
      const appError = toAppError(reason);
      if (preserveExisting && this.loaded) this.error = appError.message;
      else this.state = failed(appError.message, appError.retryable ? () => void this.load() : undefined);
      return false;
    }
  }
}

export const projects = new ProjectsStore();

function readSelectedId(): string | null {
  const value = globalThis.localStorage?.getItem(SELECTED_PROJECT_KEY);
  return value && value.startsWith('project:') ? value : null;
}
