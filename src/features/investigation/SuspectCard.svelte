<script lang="ts">
  import Card from '../../components/ui/Card.svelte';
  import { formatAbsoluteTime } from '../../lib/formatting/time';
  import ChainSteps from './ChainSteps.svelte';
  import ConfidenceBadge from './ConfidenceBadge.svelte';
  import type { SuspectView } from './investigation-types';

  interface Props { suspect: SuspectView; rank: number; }
  let { suspect, rank }: Props = $props();
</script>

<Card level={2} class="h-full" data-testid="suspect-card">
  <div class="flex items-start justify-between gap-3">
    <div class="min-w-0">
      <p class="text-base font-medium text-text-primary">
        <span class="mr-2 font-tabular text-text-muted">{rank}.</span>{suspect.event.title}
      </p>
      <p class="mt-1 text-xs text-text-muted">
        {formatAbsoluteTime(suspect.event.timestamp)}
        {#if suspect.explanation.relativeTime} · {suspect.explanation.relativeTime}{/if}
      </p>
    </div>
    <ConfidenceBadge confidence={suspect.evidence} />
  </div>

  <p class="mt-3 text-sm text-text-primary">{suspect.explanation.confidenceSentence}</p>

  {#if suspect.explanation.whyRanked.length > 0}
    <div class="mt-3">
      <p class="text-caps text-text-muted">Why this ranked</p>
      <ul class="mt-1 list-disc space-y-1 pl-5 text-sm text-text-secondary">
        {#each suspect.explanation.whyRanked as reason (reason)}<li>{reason}</li>{/each}
      </ul>
    </div>
  {/if}

  <ChainSteps steps={suspect.chain} />

  <div class="mt-3 rounded-md bg-surface-3 px-3 py-2 text-sm text-text-secondary">
    <span class="font-medium text-text-primary">What to try:</span>
    {suspect.explanation.remediation}
  </div>
</Card>
