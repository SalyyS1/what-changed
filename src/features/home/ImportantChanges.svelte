<script lang="ts">
  import { eventTitle } from '../../lib/formatting/eventCopy';
  import { formatClock } from '../../lib/formatting/time';
  import { eventDetails } from '../../lib/stores/eventDetails.svelte';
  import type { EventSummary } from '../../lib/tauri/bindings';
  interface Props { events: EventSummary[]; }
  let { events }: Props = $props();
</script>

<section aria-labelledby="important-changes-title">
  <h2 id="important-changes-title" class="mb-2 text-caps text-text-muted">Important changes</h2>
  {#if events.length === 0}
    <p class="rounded-lg border border-border-subtle bg-surface-1 px-4 py-5 text-sm text-text-muted">No important changes in this range.</p>
  {:else}
    <ul class="m-0 flex list-none flex-col gap-1 p-0">
      {#each events as event (event.id)}
        <li>
          <button
            type="button"
            class="flex w-full items-baseline justify-between gap-4 rounded-md px-3 py-2 text-left hover:bg-surface-2"
            onclick={(current) => void eventDetails.show(event.id, current.currentTarget)}
          >
            <span class="truncate text-base text-text-primary">{eventTitle(event)}</span>
            <time class="shrink-0 text-xs text-text-muted font-tabular">{formatClock(event.timestamp)}</time>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>
