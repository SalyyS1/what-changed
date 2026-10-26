<script lang="ts">
  import SystemMapFrame from '../../components/system/SystemMapFrame.svelte';
  import { ready } from '../../lib/types/async-state';
  import { systemMapSample } from './system-map-dev-fixture';

  let frameMedian = $state<number | null>(null);
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-hidden px-6 py-5" data-testid="system-map-perf">
  <h1 class="font-display text-xl text-text-primary">System Map performance</h1>
  <p class="mb-4 text-sm text-text-secondary">400 nodes · low quality tier · end-to-end median after 300 frame intervals.</p>
  <div class="@container/shell relative flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border-active bg-background">
    <SystemMapFrame data={ready(systemMapSample(400))} mode="3d" quality="low" onmetrics={(median) => (frameMedian = median)} />
    {#if frameMedian !== null}
      <p class="absolute top-3 right-3 z-40 rounded bg-surface-1 px-2 py-1 text-xs text-text-muted" data-testid="system-map-frame-median">300-frame median: {frameMedian.toFixed(2)} ms</p>
    {/if}
  </div>
</div>
