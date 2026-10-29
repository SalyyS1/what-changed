<script lang="ts">
  import Card from '../../components/ui/Card.svelte';
  import { NO_CAUSE_COPY, type NoCauseReason } from './investigation-types';

  interface Props { reason: NoCauseReason; }
  let { reason }: Props = $props();
</script>

<Card level={2} class="text-center" data-testid="no-cause-state">
  <p class="font-display text-lg text-text-primary">No obvious cause</p>
  <p class="mt-1 text-sm text-text-secondary">{NO_CAUSE_COPY[reason]}</p>
  <p class="mt-2 text-xs text-text-muted">No change is being blamed without enough evidence.</p>
</Card>
