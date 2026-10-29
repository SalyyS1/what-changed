<script lang="ts">
  import SystemMapFrame from '../../components/system/SystemMapFrame.svelte';
  import { empty, failed, loading, ready } from '../../lib/types/async-state';
  import type { AsyncState } from '../../lib/types/async-state';
  import type { SystemMapData } from '../../lib/tauri/bindings';
  import type { QualityTier } from '../../three/changeMap/quality';
  import { systemMapSample } from './system-map-dev-fixture';

  const disabledKind = systemMapSample(18);
  disabledKind.kind_availability = disabledKind.kind_availability.map((item) => item.kind === 'container'
    ? { ...item, count: 0, available: false, reason: 'No Docker containers recorded — the Docker collector is off' }
    : item);
  const states: Array<{ label: string; state: AsyncState<SystemMapData>; mode: '2d' | '3d'; quality: QualityTier }> = [
    { label: 'Loading', state: loading(), mode: '2d', quality: 'high' },
    { label: 'Empty', state: empty(), mode: '2d', quality: 'high' },
    { label: 'Error', state: failed('The system map could not be loaded.'), mode: '2d', quality: 'high' },
    { label: 'Populated 3D', state: ready(disabledKind), mode: '3d', quality: 'high' },
    { label: 'Forced 2D', state: ready(systemMapSample(18)), mode: '2d', quality: 'high' },
    { label: 'Capped 400 · low quality', state: ready(systemMapSample(1832)), mode: '3d', quality: 'low' },
  ];
</script>

<section class="mb-8">
  <h2 class="mb-2 text-caps text-text-muted">System Map states</h2>
  <div class="grid grid-cols-1 gap-4 @[1200px]/shell:grid-cols-2">
    {#each states as item (item.label)}
      <div class="flex flex-col gap-1">
        <span class="text-xs text-text-muted">{item.label}</span>
        <div class="@container/shell flex h-90 flex-col overflow-hidden rounded-lg border border-border-subtle bg-background">
          <SystemMapFrame data={item.state} mode={item.mode} quality={item.quality} />
        </div>
      </div>
    {/each}
  </div>
  <h3 class="mt-5 mb-2 text-caps text-text-muted">System Map · 900×600</h3>
  <div class="@container/shell flex h-150 w-225 max-w-full flex-col overflow-hidden rounded-lg border border-border-active bg-background">
    <SystemMapFrame data={ready(systemMapSample(18))} mode="2d" />
  </div>
</section>
