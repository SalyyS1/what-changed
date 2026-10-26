import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import type { EntityRef, EntityType, SystemMapData } from '../tauri/bindings';
import { getSystemMap, toAppError } from '../tauri/client';

interface SystemMapDependencies {
  fetch: (focus: EntityRef | null, depth: number, kinds: EntityType[]) => Promise<SystemMapData>;
}

export interface SystemMapUrlState {
  focus: EntityRef | null;
  depth: number;
  kinds: EntityType[];
}

const DEPTH_MIN = 1;
const DEPTH_MAX = 3;
const FILTER_KINDS: readonly EntityType[] = ['app', 'process', 'service', 'port', 'container', 'adapter', 'file'];
const ENTITY_TYPES = new Set<EntityType>([
  'process', 'port', 'service', 'app', 'driver', 'file', 'adapter',
  'container', 'image', 'docker_volume', 'docker_network', 'project', 'update', 'task', 'system',
]);

function isEntityType(value: string | null): value is EntityType {
  return value !== null && ENTITY_TYPES.has(value as EntityType);
}

function isValidEntityId(id: string, type: EntityType): boolean {
  if (id.length > 2_000 || id.trim() !== id) return false;
  const separator = id.indexOf(':');
  if (separator < 1 || id.slice(0, separator) !== type) return false;
  const suffix = id.slice(separator + 1);
  if (suffix.length === 0 || /\p{Cc}/u.test(suffix)) return false;
  if (type === 'process' || type === 'file' || type === 'project') return /^[0-9a-f]{16}$/u.test(suffix);
  if (type === 'system') return suffix === 'local';
  if (type === 'container') return [...suffix].length <= 64;
  if (type === 'port') {
    const match = /^(tcp|udp):(.+):(\d+)$/u.exec(suffix);
    if (!match) return false;
    const port = Number(match[3]);
    return Number.isInteger(port) && port >= 0 && port <= 65_535;
  }
  return true;
}

export function clampSystemMapDepth(value: number): number {
  return Math.max(DEPTH_MIN, Math.min(DEPTH_MAX, Math.round(value) || 2));
}

export function parseSystemMapHash(hash: string): SystemMapUrlState {
  const query = hash.split('?')[1] ?? '';
  const params = new URLSearchParams(query);
  const entityId = params.get('focus');
  const rawType = params.get('type');
  const entityType = isEntityType(rawType) ? rawType : null;
  const kinds = params.getAll('kind').filter(isEntityType);
  return {
    focus: entityId && entityType && isValidEntityId(entityId, entityType) ? { entity_id: entityId, entity_type: entityType } : null,
    depth: clampSystemMapDepth(Number(params.get('depth') ?? 2)),
    kinds: [...new Set(kinds)].sort(),
  };
}

export function systemMapHash(state: SystemMapUrlState): string {
  const params = new URLSearchParams();
  if (state.focus) {
    params.set('focus', state.focus.entity_id);
    params.set('type', state.focus.entity_type);
  }
  params.set('depth', String(clampSystemMapDepth(state.depth)));
  for (const kind of [...state.kinds].sort()) params.append('kind', kind);
  return `#/system-map?${params.toString()}`;
}

export class SystemMapStore {
  state = $state<AsyncState<SystemMapData>>(loading());
  focus = $state<EntityRef | null>(null);
  depth = $state(2);
  kinds = $state<EntityType[]>([]);
  hoveredId = $state<string | null>(null);
  #generation = 0;
  #deps: SystemMapDependencies;

  constructor(deps: SystemMapDependencies = { fetch: getSystemMap }) {
    this.#deps = deps;
  }

  restore(hash = globalThis.location?.hash ?? '#/system-map'): void {
    const restored = parseSystemMapHash(hash);
    this.focus = restored.focus;
    this.depth = restored.depth;
    this.kinds = restored.kinds;
  }

  writeUrl(historyLike: Pick<History, 'replaceState'> = history): void {
    historyLike.replaceState(null, '', systemMapHash(this));
  }

  async load(): Promise<boolean> {
    const generation = ++this.#generation;
    this.state = loading();
    try {
      const data = await this.#deps.fetch(this.focus, this.depth, [...this.kinds]);
      if (generation !== this.#generation) return false;
      this.focus = data.focus;
      this.state = data.nodes.length === 0 ? empty() : ready(data);
      return true;
    } catch (reason) {
      if (generation !== this.#generation) return false;
      const error = toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => void this.load() : undefined);
      return false;
    }
  }

  async setFocus(focus: EntityRef | null): Promise<void> {
    this.focus = focus;
    this.hoveredId = null;
    this.writeUrl();
    await this.load();
  }

  async setDepth(depth: number): Promise<void> {
    this.depth = clampSystemMapDepth(depth);
    this.writeUrl();
    await this.load();
  }

  async toggleKind(kind: EntityType): Promise<void> {
    const current = this.kinds.length === 0 ? [...FILTER_KINDS] : this.kinds;
    this.kinds = current.includes(kind)
      ? current.filter((candidate) => candidate !== kind)
      : [...current, kind].sort();
    this.writeUrl();
    await this.load();
  }

  hover(entityId: string | null): void { this.hoveredId = entityId; }
  cancel(): void { this.#generation += 1; }
}

export const systemMap = new SystemMapStore();
