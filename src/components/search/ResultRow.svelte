<script lang="ts">
  import { eventTitle } from '../../lib/formatting/eventCopy';
  import { formatClock } from '../../lib/formatting/time';
  import type { SearchResultDto } from '../../lib/tauri/bindings';
  interface Props {
    result: SearchResultDto;
    index: number;
    selected: boolean;
    onselect: (result: SearchResultDto, opener: HTMLElement) => void;
  }
  let { result, index, selected, onselect }: Props = $props();
  const title = $derived(result.kind === 'checkpoint' ? result.checkpoint.name : eventTitle(result.event));
  const detail = $derived(result.kind === 'checkpoint' ? formatClock(result.checkpoint.created_at) : `${result.event.entity_display_name} · ${formatClock(result.event.timestamp)}`);
</script>

<button
  id={`search-result-${index}`}
  type="button"
  role="option"
  aria-selected={selected}
  class="flex w-full items-center justify-between gap-4 rounded-md px-3 py-2 text-left {selected ? 'bg-accent/15' : 'hover:bg-surface-2'}"
  onclick={(current) => onselect(result, current.currentTarget)}
  data-testid="search-result"
>
  <span class="min-w-0 truncate text-sm text-text-primary">{title}</span>
  <span class="shrink-0 truncate text-xs text-text-muted font-tabular">{detail}</span>
</button>
