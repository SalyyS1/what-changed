import type { CollectorCadenceDto, CollectorCost, CollectorState } from '../../lib/tauri/bindings';

export type CollectorTone = 'neutral' | 'success' | 'warning' | 'danger' | 'info';

export function collectorStatePresentation(state: CollectorState): { label: string; tone: CollectorTone } {
  switch (state) {
    case 'running':
    case 'idle': return { label: 'Running', tone: 'success' };
    case 'paused': return { label: 'Paused', tone: 'warning' };
    case 'failed': return { label: 'Failed', tone: 'danger' };
    case 'degraded': return { label: 'Degraded', tone: 'warning' };
    case 'disabled': return { label: 'Disabled', tone: 'neutral' };
    case 'unavailable': return { label: 'Unavailable', tone: 'neutral' };
  }
}

export function collectorCostPresentation(cost: CollectorCost): { label: string; tone: CollectorTone } {
  if (cost === 'cheap') return { label: 'Low impact', tone: 'neutral' };
  if (cost === 'moderate') return { label: 'Moderate impact', tone: 'info' };
  return { label: 'Higher impact', tone: 'warning' };
}

function intervalLabel(milliseconds: number): string {
  if (milliseconds < 1_000) return `every ${Math.max(1, Math.round(milliseconds))} ms`;
  if (milliseconds < 60_000 || milliseconds % 60_000 !== 0) return `every ${Math.round(milliseconds / 1_000)} s`;
  const minutes = Math.round(milliseconds / 60_000);
  return `every ${minutes} ${minutes === 1 ? 'minute' : 'minutes'}`;
}

export function collectorCadenceLabel(cadence: CollectorCadenceDto): string {
  if (cadence.kind === 'notify') return 'Live notifications';
  if (cadence.kind === 'poll') return `Checks ${intervalLabel(cadence.interval_ms)}`;
  return `Live notifications + sweep ${intervalLabel(cadence.interval_ms)}`;
}

export function durationLabel(milliseconds: number | null): string {
  if (milliseconds === null) return '—';
  if (milliseconds < 1) return '<1 ms';
  if (milliseconds < 1_000) return `${Math.round(milliseconds)} ms`;
  return `${(milliseconds / 1_000).toFixed(milliseconds < 10_000 ? 1 : 0)} s`;
}
