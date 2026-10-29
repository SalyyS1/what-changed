<script lang="ts">
  import { technicalEventCopy } from '../../lib/formatting/eventCopy';
  import type { SystemEvent } from '../../lib/tauri/bindings';
  import Button from '../ui/Button.svelte';
  import Icon from '../ui/Icon.svelte';

  interface Props { event: SystemEvent; open: boolean; ontoggle: () => void; }
  let { event, open, ontoggle }: Props = $props();
</script>

<Button
  variant="ghost"
  size="sm"
  aria-expanded={open}
  aria-controls="event-technical-details"
  onclick={ontoggle}
>
  <Icon name={open ? 'chevron-down' : 'chevron-right'} size={14} />
  {open ? 'Hide technical details' : 'Show technical details'}
</Button>
{#if open}
  <div id="event-technical-details" class="mt-2 rounded-lg border border-border-subtle bg-surface-2 p-3">
    <dl class="m-0 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-2 text-xs">
      <dt class="text-text-muted">Entity id</dt><dd class="selectable m-0 break-all font-mono text-text-secondary">{event.entity_id}</dd>
      <dt class="text-text-muted">Entity type</dt><dd class="m-0 font-mono text-text-secondary">{event.entity_type}</dd>
      <dt class="text-text-muted">Event type</dt><dd class="m-0 font-mono text-text-secondary">{event.event_type}</dd>
      <dt class="text-text-muted">Technical summary</dt><dd class="selectable m-0 break-words font-mono text-text-secondary">{technicalEventCopy(event)}</dd>
      <dt class="text-text-muted">Metadata</dt><dd class="selectable m-0 break-words font-mono text-text-secondary">{event.metadata === null ? 'Not recorded' : JSON.stringify(event.metadata)}</dd>
      <dt class="text-text-muted">Related ids</dt><dd class="selectable m-0 break-words font-mono text-text-secondary">{event.related_entity_ids.join(', ') || 'None'}</dd>
    </dl>
  </div>
{/if}
