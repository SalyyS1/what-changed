import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import type { SearchResultDto } from '../tauri/bindings';
import { searchEvents, toAppError } from '../tauri/client';

export type SearchGroupName =
  | 'Applications'
  | 'Files'
  | 'Ports'
  | 'Services'
  | 'Drivers'
  | 'Docker'
  | 'Checkpoints'
  | 'Events';

export interface SearchGroup {
  name: SearchGroupName;
  results: SearchResultDto[];
}

const groupOrder: readonly SearchGroupName[] = [
  'Applications',
  'Files',
  'Ports',
  'Services',
  'Drivers',
  'Docker',
  'Checkpoints',
  'Events',
];

export function searchGroupName(result: SearchResultDto): SearchGroupName {
  if (result.kind === 'checkpoint') return 'Checkpoints';
  switch (result.event.entity_type) {
    case 'app': return 'Applications';
    case 'file': return 'Files';
    case 'port': return 'Ports';
    case 'service': return 'Services';
    case 'driver': return 'Drivers';
    case 'container':
    case 'image':
    case 'docker_volume':
    case 'docker_network': return 'Docker';
    default: return 'Events';
  }
}

export function groupSearchResults(results: SearchResultDto[]): SearchGroup[] {
  return groupOrder.flatMap((name) => {
    const matches = results.filter((result) => searchGroupName(result) === name);
    return matches.length > 0 ? [{ name, results: matches }] : [];
  });
}

interface SearchDependencies {
  search: (query: string, limit: number) => Promise<SearchResultDto[]>;
}

export class SearchStore {
  open = $state(false);
  query = $state('');
  state = $state<AsyncState<SearchGroup[]>>(empty());
  activeIndex = $state(0);
  searching = $state(false);
  #deps: SearchDependencies;
  #debounce: ReturnType<typeof setTimeout> | null = null;
  #loadingDelay: ReturnType<typeof setTimeout> | null = null;
  #request = 0;

  constructor(dependencies: SearchDependencies = { search: searchEvents }) {
    this.#deps = dependencies;
  }

  get flatResults(): SearchResultDto[] {
    return this.state.status === 'ready'
      ? this.state.data.flatMap((group) => group.results)
      : [];
  }

  show(): void {
    this.open = true;
  }

  close(): void {
    this.#request += 1;
    this.open = false;
    this.query = '';
    this.state = empty();
    this.activeIndex = 0;
    this.searching = false;
    this.#clearTimers();
  }

  setQuery(value: string): void {
    this.query = value.slice(0, 128);
    this.activeIndex = 0;
    this.#clearTimers();
    const request = ++this.#request;
    if (this.query.trim() === '') {
      this.state = empty();
      this.searching = false;
      return;
    }
    this.searching = true;
    this.#debounce = setTimeout(() => void this.#run(this.query.trim(), request), 120);
  }

  move(delta: number): void {
    const count = this.flatResults.length;
    if (count === 0) return;
    this.activeIndex = (this.activeIndex + delta + count) % count;
  }

  #clearTimers(): void {
    if (this.#debounce) clearTimeout(this.#debounce);
    if (this.#loadingDelay) clearTimeout(this.#loadingDelay);
    this.#debounce = null;
    this.#loadingDelay = null;
  }

  async #run(query: string, request: number): Promise<void> {
    this.#loadingDelay = setTimeout(() => {
      if (request === this.#request) this.state = loading();
    }, 300);
    try {
      const results = await this.#deps.search(query, 100);
      if (request !== this.#request) return;
      this.#clearTimers();
      this.searching = false;
      this.state = results.length > 0 ? ready(groupSearchResults(results)) : empty();
    } catch (reason) {
      if (request !== this.#request) return;
      this.#clearTimers();
      this.searching = false;
      const error = toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => this.setQuery(this.query) : undefined);
    }
  }
}

export const search = new SearchStore();
