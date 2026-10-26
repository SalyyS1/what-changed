<script lang="ts">
  import FilterChips from '../../components/timeline/FilterChips.svelte';
  import RangeTabs from '../../components/timeline/RangeTabs.svelte';
  import Button from '../../components/ui/Button.svelte';
  import type { TimelineFilterKey, TimelineRangeKey } from '../../lib/stores/timeline.svelte';
  import CustomRangePicker from './CustomRangePicker.svelte';

  interface Props {
    range: TimelineRangeKey;
    filter: TimelineFilterKey;
    customFrom: number;
    customTo: number;
    onrange: (range: TimelineRangeKey) => void;
    onfilter: (filter: TimelineFilterKey) => void;
    oncustom: (from: number, to: number) => void;
    onmap: () => void;
  }
  let { range, filter, customFrom, customTo, onrange, onfilter, oncustom, onmap }: Props = $props();
</script>

<div class="flex shrink-0 flex-col gap-3 border-b border-border-subtle px-6 pb-3">
  <div class="flex items-center justify-between gap-3">
    <RangeTabs value={range} onchange={onrange} />
    <Button size="sm" data-change-map-trigger="timeline" onclick={onmap}>Map</Button>
  </div>
  <FilterChips value={filter} onchange={onfilter} />
  {#if range === 'custom'}
    <CustomRangePicker from={customFrom} to={customTo} onapply={oncustom} />
  {/if}
</div>
