<script lang="ts">
  import type { TimelineRangeKey } from '../../lib/stores/timeline.svelte';
  import Tabs from '../ui/Tabs.svelte';

  interface Props { value: TimelineRangeKey; onchange: (value: TimelineRangeKey) => void; }
  let { value, onchange }: Props = $props();
  const items = [
    { value: 'today', label: 'Today' },
    { value: 'yesterday', label: 'Yesterday' },
    { value: 'last7_days', label: 'Last 7 days' },
    { value: 'custom', label: 'Custom' },
  ];
</script>

<Tabs items={items} {value} label="Time range" onchange={(next) => onchange(next as TimelineRangeKey)} />
