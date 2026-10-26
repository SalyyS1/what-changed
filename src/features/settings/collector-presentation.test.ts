import { describe, expect, it } from 'vitest';
import { collectorCadenceLabel, collectorStatePresentation, durationLabel } from './collector-presentation';

describe('collector presentation', () => {
  it('uses calm user-facing labels for every runtime state', () => {
    expect(['running', 'paused', 'failed', 'disabled', 'unavailable', 'idle', 'degraded'].map((state) =>
      collectorStatePresentation(state as Parameters<typeof collectorStatePresentation>[0]).label,
    )).toEqual(['Running', 'Paused', 'Failed', 'Disabled', 'Unavailable', 'Running', 'Degraded']);
  });

  it('formats each cadence without exposing wire vocabulary', () => {
    expect(collectorCadenceLabel({ kind: 'notify' })).toBe('Live notifications');
    expect(collectorCadenceLabel({ kind: 'poll', interval_ms: 5_000 })).toBe('Checks every 5 s');
    expect(collectorCadenceLabel({ kind: 'notify_plus_sweep', interval_ms: 300_000 })).toBe('Live notifications + sweep every 5 minutes');
  });

  it('keeps durations compact', () => {
    expect(durationLabel(null)).toBe('—');
    expect(durationLabel(12)).toBe('12 ms');
    expect(durationLabel(1_250)).toBe('1.3 s');
  });
});
