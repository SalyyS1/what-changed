<script lang="ts">
  import { collectorLabel, eventTitle } from '../../lib/formatting/eventCopy';
  import { formatAbsoluteTime } from '../../lib/formatting/time';
  import { formatRelativeTime } from '../../lib/formatting/relative';
  import { eventDetails as sharedDetails, type EventDetailsStore } from '../../lib/stores/eventDetails.svelte';
  import StateShell from '../state/StateShell.svelte';
  import Drawer from '../ui/Drawer.svelte';
  import BeforeAfterTable from './BeforeAfterTable.svelte';
  import ConfidenceBadge from './ConfidenceBadge.svelte';
  import ImportanceChip from './ImportanceChip.svelte';
  import RelatedEvents from './RelatedEvents.svelte';
  import TechnicalDetails from './TechnicalDetails.svelte';

  interface Props { controller?: EventDetailsStore; }
  let { controller = sharedDetails }: Props = $props();
  let previouslyOpen = $state(false);
  const title = $derived(controller.state.status === 'ready' ? controller.state.data.event.entity_display_name : 'Event details');

  $effect(() => {
    const open = controller.open;
    if (previouslyOpen && !open) {
      const target = controller.returnFocus;
      requestAnimationFrame(() => target?.focus());
    }
    previouslyOpen = open;
  });
</script>

<Drawer bind:open={controller.open} {title}>
  <StateShell
    state={controller.state}
    emptyTitle="This event is no longer available."
    errorTitle="Event details could not be loaded"
    loadingRows={5}
  >
    {#snippet children(details)}
      <article class="space-y-5">
        {#if controller.snapshotOnly}
          <p class="rounded-md bg-info/10 px-3 py-2 text-sm text-info" data-testid="snapshot-retention-notice">
            Detailed before/after data is no longer retained. This summary comes from the saved investigation.
          </p>
        {/if}
        <div>
          <div class="mb-2 flex flex-wrap items-center gap-2">
            <ImportanceChip importance={details.event.importance} />
            <ConfidenceBadge confidence={details.event.confidence} />
          </div>
          <p class="font-display text-lg text-text-primary">{eventTitle(details.event)}</p>
        </div>

        <dl class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2 text-sm">
          <dt class="text-text-muted">When</dt>
          <dd class="m-0 text-text-primary">{formatAbsoluteTime(details.event.timestamp)} <span class="text-text-muted">({formatRelativeTime(details.event.timestamp)})</span></dd>
          <dt class="text-text-muted">Affected</dt>
          <dd class="m-0 text-text-primary">{details.event.entity_display_name}</dd>
          <dt class="text-text-muted">Detected by</dt>
          <dd class="m-0 text-text-primary">{collectorLabel(details.event.source_collector)}</dd>
          <dt class="text-text-muted">Occurrences</dt>
          <dd class="m-0 text-text-primary">This happened {details.prior_occurrences} {details.prior_occurrences === 1 ? 'time' : 'times'} before</dd>
        </dl>

        {#if details.event.before !== null || details.event.after !== null}
          <section aria-labelledby="before-after-title">
            <h3 id="before-after-title" class="mb-2 text-caps text-text-muted">What changed</h3>
            <BeforeAfterTable before={details.event.before} after={details.event.after} />
          </section>
        {/if}

        <section aria-labelledby="related-title">
          <h3 id="related-title" class="mb-2 text-caps text-text-muted">Related changes</h3>
          <RelatedEvents events={details.related} onopen={(event, opener) => void controller.show(event.id, opener)} />
        </section>

        <section aria-label="Technical information">
          <TechnicalDetails event={details.event} open={controller.technicalOpen} ontoggle={() => controller.toggleTechnical()} />
        </section>
      </article>
    {/snippet}
  </StateShell>
</Drawer>
