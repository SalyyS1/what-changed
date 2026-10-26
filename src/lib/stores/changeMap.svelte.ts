import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import type { ChangeMapData, ChangeMapRequest } from '../tauri/bindings';
import { getChangeMap, toAppError } from '../tauri/client';

interface ChangeMapDependencies {
  fetch: (request: ChangeMapRequest) => Promise<ChangeMapData>;
}

export class ChangeMapStore {
  state = $state<AsyncState<ChangeMapData>>(loading());
  selectedId = $state<number | null>(null);
  hoveredId = $state<number | null>(null);
  #generation = 0;
  #deps: ChangeMapDependencies;

  constructor(deps: ChangeMapDependencies = { fetch: getChangeMap }) {
    this.#deps = deps;
  }

  async load(request: ChangeMapRequest): Promise<boolean> {
    const generation = ++this.#generation;
    this.state = loading();
    this.hoveredId = null;
    try {
      const data = await this.#deps.fetch(request);
      if (generation !== this.#generation) return false;
      this.selectedId = data.nodes.some((node) => node.event_id === this.selectedId) ? this.selectedId : null;
      this.state = data.nodes.length === 0 ? empty() : ready(data);
      return true;
    } catch (reason) {
      if (generation !== this.#generation) return false;
      const error = toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => void this.load(request) : undefined);
      return false;
    }
  }

  select(eventId: number): void { this.selectedId = eventId; }
  hover(eventId: number | null): void { this.hoveredId = eventId; }
  cancel(): void { this.#generation += 1; }
}

export const changeMap = new ChangeMapStore();
