<script lang="ts">
  import type { TimelineFilterKey } from '../../lib/stores/timeline.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import DropdownMenu from '../ui/DropdownMenu.svelte';
  import Icon from '../ui/Icon.svelte';

  interface Props { value: TimelineFilterKey; onchange: (value: TimelineFilterKey) => void; }
  let { value, onchange }: Props = $props();
  const filters: readonly { id: TimelineFilterKey; label: string }[] = [
    { id: 'all', label: 'All' }, { id: 'apps', label: 'Apps' },
    { id: 'windows', label: 'Windows' }, { id: 'network', label: 'Network' },
    { id: 'drivers', label: 'Drivers' }, { id: 'files', label: 'Files' },
    { id: 'development', label: 'Development' }, { id: 'docker', label: 'Docker' },
    { id: 'important', label: 'Important only' },
  ];
  const activeLabel = $derived(filters.find((filter) => filter.id === value)?.label ?? 'All');
</script>

<div data-testid="timeline-filter-chips" data-collapse-below="1040" class="hidden flex-wrap gap-1.5 @[1040px]/shell:flex" aria-label="Timeline filters">
  {#each filters as filter (filter.id)}
    <Chip selected={value === filter.id} onclick={() => onchange(filter.id)}>{filter.label}</Chip>
  {/each}
</div>
<div data-testid="timeline-filter-menu" data-collapse-below="1040" class="@[1040px]/shell:hidden">
  <DropdownMenu items={filters} onselect={(id) => onchange(id as TimelineFilterKey)} align="start">
    {#snippet trigger(props)}
      <Button {...props} variant="secondary" size="sm">
        <Icon name="timeline" size={14} /> Filter: {activeLabel} <Icon name="chevron-down" size={13} />
      </Button>
    {/snippet}
  </DropdownMenu>
</div>
