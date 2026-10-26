<script lang="ts">
  import ChangeMapFrame from '../../components/visualization/ChangeMapFrame.svelte';
  import type { ChangeMapData } from '../../lib/tauri/bindings';
  import { empty, failed, loading, ready } from '../../lib/types/async-state';
  import type { AsyncState } from '../../lib/types/async-state';
  import type { QualityTier } from '../../three/changeMap/quality';
  import { changeMapDevRequest as request, changeMapSample as sample } from './change-map-dev-fixture';

  const states: Array<{ label: string; state: AsyncState<ChangeMapData>; mode: '2d' | '3d'; quality: QualityTier }> = [
    { label: 'Loading', state: loading(), mode: '2d' as const, quality: 'high' },
    { label: 'Empty', state: empty(), mode: '2d' as const, quality: 'high' },
    { label: 'Error', state: failed('The map could not be loaded.'), mode: '2d' as const, quality: 'high' },
    { label: 'Populated 3D', state: ready(sample(12)), mode: '3d' as const, quality: 'high' },
    { label: 'Forced 2D', state: ready(sample(12)), mode: '2d' as const, quality: 'high' },
    { label: 'Capped 3,000 · low quality', state: ready(sample(3000)), mode: '3d' as const, quality: 'low' },
  ];
</script>

<section class="mb-8">
  <h2 class="mb-2 text-caps text-text-muted">Change Map contextual states</h2>
  <div class="grid grid-cols-1 gap-4 @[1200px]/shell:grid-cols-2">
    {#each states as item (item.label)}
      <div class="flex flex-col gap-1">
        <span class="text-xs text-text-muted">{item.label}</span>
        <div class="@container/shell flex h-90 flex-col overflow-hidden rounded-lg border border-border-subtle bg-background">
          <ChangeMapFrame {request} subtitle="API cannot connect" onclose={() => {}} data={item.state} mode={item.mode} quality={item.quality} />
        </div>
      </div>
    {/each}
  </div>
  <h3 class="mt-5 mb-2 text-caps text-text-muted">Change Map · 900×600</h3>
  <div class="@container/shell flex h-150 w-225 max-w-full flex-col overflow-hidden rounded-lg border border-border-active bg-background">
    <ChangeMapFrame {request} subtitle="API cannot connect" onclose={() => {}} data={ready(sample(12))} mode="2d" />
  </div>
</section>
