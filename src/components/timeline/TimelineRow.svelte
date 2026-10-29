<script lang="ts">
  import { displayValue, eventTitle } from '../../lib/formatting/eventCopy';
  import { formatClock } from '../../lib/formatting/time';
  import type { EventSummary } from '../../lib/tauri/bindings';
  import Badge from '../ui/Badge.svelte';

  interface Props {
    event: EventSummary;
    entering?: boolean;
    onopen: (event: EventSummary, opener: HTMLElement) => void;
  }

  let { event, entering = false, onopen }: Props = $props();
  const tone = $derived(event.importance === 'critical' ? 'danger' : event.importance === 'important' ? 'warning' : event.importance === 'notable' ? 'info' : 'neutral');
  const delta = $derived(event.before !== null || event.after !== null
    ? `${displayValue(event.before)} → ${displayValue(event.after)}`
    : null);
</script>

<button
  type="button"
  class="grid h-18 w-full grid-cols-[3.5rem_minmax(0,1fr)_auto] items-center gap-3 border-b border-border-subtle px-6 text-left hover:bg-surface-1 {entering ? 'wc-live-enter' : ''}"
  aria-label={`View details: ${eventTitle(event)}`}
  onclick={(current) => onopen(event, current.currentTarget)}
  data-testid="timeline-row"
>
  <time class="text-xs text-text-muted font-tabular" datetime={new Date(event.timestamp).toISOString()}>
    {formatClock(event.timestamp)}
  </time>
  <span class="min-w-0">
    <span class="block truncate text-base text-text-primary" title={eventTitle(event)}>{eventTitle(event)}</span>
    <span class="mt-0.5 block truncate text-xs text-text-muted">{event.entity_display_name}</span>
    {#if delta}
      <span class="block truncate font-mono text-xs text-text-secondary" title={delta}>{delta}</span>
    {/if}
  </span>
  <span class="flex items-center gap-3">
    {#if event.importance !== 'informational'}
      <Badge tone={tone}>{event.importance}</Badge>
    {/if}
    <span class="text-xs text-accent">View details</span>
  </span>
</button>

<style>
  .wc-live-enter { animation: wc-fade-in var(--t-fast) var(--ease-standard); }
</style>
