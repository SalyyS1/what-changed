/**
 * Bulk fixtures for the harness's `large` state: a day of a busy machine,
 * not lorem. Generated from a small vocabulary with a fixed sequence so the
 * page is identical on every load and a screenshot diff means something.
 *
 * The timeline fixture mirrors one maximum backend page (200 raw rows) while
 * advertising the full 128,431-row corpus; the checkpoint screen keeps 400
 * rows to exercise its own pre-virtualization large-data state.
 */

import type { CheckpointDto } from '../../lib/tauri/bindings';
import type { TimelineSnapshot } from '../../lib/stores/timeline.svelte';
import type { Category, EntityType, EventSummary } from '../../lib/tauri/bindings';

export const LARGE_TIMELINE_ROWS = 200;
export const LARGE_CHECKPOINT_ROWS = 400;

const processes = ['node.exe', 'Code.exe', 'Discord.exe', 'msedge.exe', 'docker.exe', 'pwsh.exe', 'rustc.exe', 'bun.exe'];
const services = ['Docker Desktop Service', 'Windows Update', 'Print Spooler', 'WSL Service', 'Windows Search'];
const drivers = ['Realtek audio', 'NVIDIA display', 'Intel Wi-Fi', 'Bluetooth radio'];
const ports = [3000, 5173, 5432, 6379, 8080, 27017];

// Small linear congruential generator: deterministic, dependency-free, and
// good enough to make a list look lived-in rather than patterned.
function sequence(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
    return s / 0x1_0000_0000;
  };
}

function pick<T>(next: () => number, from: readonly T[]): T {
  return from[Math.floor(next() * from.length)] as T;
}

export function largeTimeline(): TimelineSnapshot {
  const next = sequence(0x5eed);
  const rows: EventSummary[] = [];
  for (let i = 0; i < LARGE_TIMELINE_ROWS; i++) {
    const kind = next();
    const id = i + 1;
    let category: Category = 'process';
    let entityType: EntityType = 'process';
    let entity = '';
    let title = '';
    if (kind < 0.45) {
      const pid = 1000 + Math.floor(next() * 60000);
      entity = pick(next, processes);
      title = `${entity} started (PID ${pid})`;
    } else if (kind < 0.7) {
      const port = pick(next, ports);
      const owner = pick(next, processes);
      category = 'network'; entityType = 'port'; entity = String(port);
      title = `Port ${port} ${next() < 0.5 ? 'now listening' : 'closed'} (${owner})`;
    } else if (kind < 0.85) {
      category = 'service'; entityType = 'service'; entity = pick(next, services);
      title = `Service "${entity}" ${next() < 0.5 ? 'started' : 'stopped'}`;
    } else if (kind < 0.95) {
      const major = 6 + Math.floor(next() * 3);
      const patch = Math.floor(next() * 20);
      category = 'driver'; entityType = 'driver'; entity = pick(next, drivers);
      title = `${entity} driver updated ${major}.0.${patch} → ${major}.0.${patch + 1}`;
    } else {
      category = 'update'; entityType = 'update'; entity = `KB50${31000 + Math.floor(next() * 999)}`;
      title = `Windows Update ${entity} installed`;
    }
    rows.push({ id, timestamp: Date.now() - i * 45_000, category, event_type: 'updated', entity_type: entityType, entity_id: `${entityType}:${entity}`, entity_display_name: entity, severity: 'info', importance: category === 'driver' ? 'important' : 'informational', confidence: 'high', source_collector: 'synthetic', before: null, after: null, field: null, user_friendly_summary: title, occurred_at_is_uncertain: false });
  }
  return { items: rows.map((event) => ({ kind: 'event' as const, event })), loadedRows: 200, totalCount: 128_431, hasMore: true };
}

export function largeCheckpoints(): CheckpointDto[] {
  const next = sequence(0xc0ffee);
  const days = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  const rows: CheckpointDto[] = [];
  for (let i = 0; i < LARGE_CHECKPOINT_ROWS; i++) {
    const manual = next() < 0.15;
    const day = days[Math.floor(i / 8) % days.length] as string;
    const hour = 1 + Math.floor(next() * 11);
    const createdAt = Date.now() - i * 6 * 60 * 60_000;
    const partial = !manual && next() < 0.1;
    rows.push({
      id: i + 1,
      name: manual ? `Before ${pick(next, drivers)} driver update` : `Checkpoint ${day} ${hour}:00`,
      kind: manual ? 'manual' : 'automatic',
      reason: manual ? 'Known-good state before a change' : 'Weekly safety checkpoint',
      created_at: createdAt,
      item_count: 2600 + Math.floor(next() * 800),
      duration_ms: 4100 + Math.floor(next() * 2200),
      partial,
      missing_collectors: partial ? ['drivers'] : [],
    });
  }
  return rows;
}
