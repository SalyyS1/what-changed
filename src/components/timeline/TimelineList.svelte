<script lang="ts">
  import { untrack } from 'svelte';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { formatDayHeader, formatHour } from '../../lib/formatting/time';
  import type { TimelineSnapshot } from '../../lib/stores/timeline.svelte';
  import type { EventSummary, TimelineItem } from '../../lib/tauri/bindings';
  import { clusterIdentity, timelineItemIdentity } from '../../lib/timeline/identity';
  import ClusterRow from './ClusterRow.svelte';
  import GroupHeader from './GroupHeader.svelte';
  import LivePill from './LivePill.svelte';
  import TimelineRow from './TimelineRow.svelte';

  type Cluster = Extract<TimelineItem, { kind: 'cluster' }>;
  type Entry = { kind: 'group'; key: string; label: string } | { kind: 'item'; key: string; item: TimelineItem };
  interface Props {
    snapshot: TimelineSnapshot;
    newCount?: number;
    entering?: ReadonlySet<string>;
    expanded?: ReadonlySet<string>;
    onopen: (event: EventSummary, opener: HTMLElement) => void;
    onexpand: (cluster: Cluster) => void;
    onmore?: () => void;
    onnewer?: () => void;
    onattop?: (atTop: boolean) => void;
    onshownew?: () => void;
  }

  let { snapshot, newCount = 0, entering = new Set(), expanded = new Set(), onopen, onexpand, onmore, onnewer, onattop, onshownew }: Props = $props();
  let viewport: HTMLDivElement;
  let scrollTop = $state(0);

  function timestamp(item: TimelineItem): number {
    return item.kind === 'event' ? item.event.timestamp : item.last_ts;
  }

  const entries = $derived.by(() => {
    const output: Entry[] = [];
    let previous = '';
    for (const item of snapshot.items) {
      const time = timestamp(item);
      const key = `${formatDayHeader(time)}:${formatHour(time)}`;
      if (key !== previous) {
        output.push({ kind: 'group', key: `group:${key}`, label: `${formatDayHeader(time)} · ${formatHour(time)}` });
        previous = key;
      }
      output.push({ kind: 'item', key: timelineItemIdentity(item), item });
    }
    return output;
  });

  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => viewport,
    estimateSize: (index) => entries[index]?.kind === 'group' ? 32 : 72,
    overscan: 8,
    initialRect: { width: 900, height: 500 },
    getItemKey: (index) => entries[index]?.key ?? index,
  });
  const virtualRows = $derived($virtualizer.getVirtualItems());
  const displayRows = $derived.by(() => {
    if (virtualRows.length > 0) return virtualRows;

    // ResizeObserver reports a zero-height viewport in the first frame (and in
    // jsdom). Keep that frame useful without ever mounting the complete list;
    // TanStack replaces these fixed-size placeholders as soon as it measures.
    let start = 0;
    return entries.slice(0, 10).map((entry, index) => {
      const size = entry.kind === 'group' ? 32 : 72;
      const row = { index, key: entry.key, start, end: start + size, size, lane: 0 };
      start += size;
      return row;
    });
  });
  const activeGroup = $derived.by(() => {
    // The sticky header occupies 32px before the virtual canvas in normal
    // flow, so remove that offset when mapping scrollTop to a virtual row.
    const visibleOffset = Math.max(0, scrollTop - 32);
    let consumed = 0;
    let firstIndex = 0;
    for (let index = 0; index < entries.length; index += 1) {
      const size = entries[index]?.kind === 'group' ? 32 : 72;
      if (consumed + size > visibleOffset) {
        firstIndex = index;
        break;
      }
      consumed += size;
    }
    for (let index = firstIndex; index >= 0; index -= 1) {
      const entry = entries[index];
      if (entry?.kind === 'group') return entry.label;
    }
    return entries.find((entry) => entry.kind === 'group')?.label ?? '';
  });

  $effect(() => {
    const count = entries.length;
    untrack(() => $virtualizer).setOptions({
      count,
      getScrollElement: () => viewport,
      estimateSize: (index) => entries[index]?.kind === 'group' ? 32 : 72,
      overscan: 8,
      initialRect: { width: 900, height: 500 },
      getItemKey: (index) => entries[index]?.key ?? index,
    });
  });

  function scrolled(): void {
    scrollTop = viewport.scrollTop;
    const atTop = scrollTop < 4;
    onattop?.(atTop);
    if (scrollTop < 320 && snapshot.hasNewer) onnewer?.();
    if (viewport.scrollTop + viewport.clientHeight >= viewport.scrollHeight - 320) onmore?.();
  }

  function showNew(): void {
    if (typeof viewport.scrollTo === 'function') viewport.scrollTo({ top: 0, behavior: 'smooth' });
    else viewport.scrollTop = 0;
    onshownew?.();
  }
</script>

<div class="relative min-h-0 flex-1" data-testid="timeline-list">
  <LivePill count={newCount} onclick={showNew} />
  <div bind:this={viewport} class="h-full overflow-y-auto" onscroll={scrolled} data-testid="timeline-viewport">
    {#if activeGroup}<GroupHeader label={activeGroup} sticky />{/if}
    <div class="relative" style:height={`${$virtualizer.getTotalSize()}px`}>
      {#each displayRows as virtualRow (virtualRow.key)}
        {@const entry = entries[virtualRow.index]}
        {#if entry}
          <div
            class="absolute top-0 left-0 w-full"
            style:height={`${virtualRow.size}px`}
            style:transform={`translateY(${virtualRow.start}px)`}
            data-index={virtualRow.index}
          >
            {#if entry.kind === 'group'}
              <GroupHeader label={entry.label} />
            {:else if entry.item.kind === 'event'}
              <TimelineRow event={entry.item.event} entering={entering.has(entry.key)} {onopen} />
            {:else}
              <ClusterRow cluster={entry.item} expanded={expanded.has(clusterIdentity(entry.item.cluster))} {onexpand} />
            {/if}
          </div>
        {/if}
      {/each}
    </div>
    <p class="px-6 py-3 text-center text-xs text-text-muted font-tabular" aria-live="polite">
      Showing {snapshot.loadedRows.toLocaleString('en-US')}{snapshot.totalCount ? ` of ${snapshot.totalCount.toLocaleString('en-US')}` : ''}
    </p>
  </div>
</div>
