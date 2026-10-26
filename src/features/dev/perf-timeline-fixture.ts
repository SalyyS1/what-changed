import type { TimelineSnapshot } from '../../lib/stores/timeline.svelte';
import { largeTimeline } from './dev-large-fixtures';

export type PerfCorpus = 10_000 | 100_000;

/** Expand the existing fixed-sequence page into an actually scrollable corpus. */
export function performanceTimeline(count: PerfCorpus): TimelineSnapshot {
  const template = largeTimeline().items;
  const first = template[0];
  if (first?.kind !== 'event') throw new Error('The large timeline fixture must contain events');
  const firstTimestamp = first.event.timestamp;
  const items = Array.from({ length: count }, (_, index) => {
    const source = template[index % template.length];
    if (source?.kind !== 'event') throw new Error('The large timeline fixture must contain events');
    const id = index + 1;
    return {
      kind: 'event' as const,
      event: {
        ...source.event,
        id,
        timestamp: firstTimestamp - index * 45_000,
        user_friendly_summary: `Fixture #${id}: ${source.event.user_friendly_summary}`,
      },
    };
  });
  return { items, loadedRows: count, totalCount: count, hasMore: false };
}
