/**
 * User-selected folder roots. Mutations are followed by a fresh list call so
 * canonical paths, availability and backend-enforced limits remain
 * authoritative in the renderer.
 */

import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import type { WatchedLocationDto } from '../tauri/bindings';
import {
  addWatchedLocation,
  listWatchedLocations,
  removeWatchedLocation,
  toAppError,
} from '../tauri/client';
import { pickDirectory } from '../tauri/dialog';

export const WATCHED_LOCATION_LIMIT = 20;

export type WatchedLocation = WatchedLocationDto;

interface WatchedLocationDependencies {
  list: () => Promise<WatchedLocation[]>;
  add: (path: string) => Promise<WatchedLocation>;
  remove: (locationId: number) => Promise<void>;
  chooseDirectory: () => Promise<string | null>;
}

const dependencies: WatchedLocationDependencies = {
  list: listWatchedLocations,
  add: addWatchedLocation,
  remove: removeWatchedLocation,
  chooseDirectory: pickDirectory,
};

export class WatchedLocationsStore {
  state = $state<AsyncState<WatchedLocation[]>>(loading());
  adding = $state(false);
  removing = $state<Set<number>>(new Set());
  /** A mutation error is shown without discarding a still-useful list. */
  error = $state<string | null>(null);
  #deps: WatchedLocationDependencies;
  #request = 0;

  constructor(overrides: WatchedLocationDependencies = dependencies) {
    this.#deps = overrides;
  }

  get rows(): WatchedLocation[] {
    return this.state.status === 'ready' ? this.state.data : [];
  }

  get loaded(): boolean {
    return this.state.status === 'ready' || this.state.status === 'empty';
  }

  get atLimit(): boolean {
    return this.rows.length >= WATCHED_LOCATION_LIMIT;
  }

  async load(): Promise<void> {
    this.error = null;
    // Reopening Settings reconciles with Rust without blanking an existing
    // list while the read is in flight.
    await this.#refresh(this.loaded);
  }

  /** Pick and add one folder. A cancelled picker deliberately does nothing. */
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

  async remove(locationId: number): Promise<boolean> {
    if (this.removing.has(locationId) || !this.rows.some((row) => row.id === locationId)) {
      return false;
    }

    this.removing = new Set(this.removing).add(locationId);
    this.error = null;
    try {
      await this.#deps.remove(locationId);
      return await this.#refresh(true);
    } catch (reason) {
      this.error = toAppError(reason).message;
      return false;
    } finally {
      const next = new Set(this.removing);
      next.delete(locationId);
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
          ? empty('Choose a folder when you want its configuration changes recorded.')
          : ready(rows);
      }
      return true;
    } catch (reason) {
      if (request !== this.#request) return true;
      const appError = toAppError(reason);
      if (preserveExisting && this.loaded) {
        this.error = appError.message;
      } else {
        this.state = failed(appError.message, appError.retryable ? () => void this.load() : undefined);
      }
      return false;
    }
  }
}

export const watchedLocations = new WatchedLocationsStore();
