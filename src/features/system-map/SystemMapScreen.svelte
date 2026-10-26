<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import SystemMapFrame from '../../components/system/SystemMapFrame.svelte';
  import { systemMap as sharedMap, type SystemMapStore } from '../../lib/stores/systemMap.svelte';
  import type { SystemMapData } from '../../lib/tauri/bindings';
  import type { AsyncState } from '../../lib/types/async-state';

  interface Props { data?: AsyncState<SystemMapData>; controller?: SystemMapStore; }
  let { data, controller = sharedMap }: Props = $props();
  let queryRestored = $state(false);

  $effect.pre(() => {
    if (queryRestored || data === undefined) return;
    queryRestored = true;
  });

  onMount(() => {
    if (data !== undefined) return;
    const syncQuery = () => {
      if (!/^#\/system-map(?:\?|$)/.test(window.location.hash)) return;
      controller.restore();
      queryRestored = true;
      void controller.load();
    };
    syncQuery();
    window.addEventListener('hashchange', syncQuery);
    return () => {
      window.removeEventListener('hashchange', syncQuery);
      controller.cancel();
    };
  });
</script>

<ScreenFrame title="System Map" subtitle="What depends on what across this PC." flush>
  {#if queryRestored}
    {#if data !== undefined}
      <SystemMapFrame {controller} {data} />
    {:else}
      <SystemMapFrame {controller} />
    {/if}
  {/if}
</ScreenFrame>
