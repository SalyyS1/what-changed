<script lang="ts">
  import { eventTitle } from '../../lib/formatting/eventCopy';
  import { formatClock } from '../../lib/formatting/time';
  import type { EventSummary } from '../../lib/tauri/bindings';
  interface Props { events: EventSummary[]; onopen: (event: EventSummary, opener: HTMLElement) => void; }
  let { events, onopen }: Props = $props();
</script>

{#if events.length === 0}
  <p class="text-sm text-text-muted">No nearby related changes.</p>
{:else}
  <ul class="m-0 list-none space-y-1 p-0">
    {#each events as event (event.id)}
      <li>
        <button
          type="button"
          class="flex w-full items-center justify-between gap-3 rounded-md px-2 py-2 text-left hover:bg-surface-2"
          onclick={(current) => onopen(event, current.currentTarget)}
        >
          <span class="min-w-0 truncate text-sm text-text-primary">{eventTitle(event)}</span>
          <time class="shrink-0 text-xs text-text-muted font-tabular">{formatClock(event.timestamp)}</time>
        </button>
      </li>
    {/each}
  </ul>
{/if}
