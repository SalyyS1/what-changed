<script lang="ts">
  import ChangeMapFrame from '../../components/visualization/ChangeMapFrame.svelte';
  import { ready } from '../../lib/types/async-state';
  import { changeMapDevRequest, changeMapSample } from './change-map-dev-fixture';
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-hidden px-6 py-5" data-testid="change-map-perf">
  <h1 class="font-display text-xl text-text-primary">Change Map performance</h1>
  <p class="mb-4 text-sm text-text-secondary">3,000 nodes · low quality tier · median appears after 300 rendered frames.</p>
  <div class="@container/shell flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border-active bg-background">
    <ChangeMapFrame
      request={changeMapDevRequest}
      subtitle="3,000-node performance fixture"
      onclose={() => {}}
      data={ready(changeMapSample(3000))}
      mode="3d"
      quality="low"
    />
  </div>
</div>
