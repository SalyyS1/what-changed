<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import TimelineList from '../../components/timeline/TimelineList.svelte';
  import ChangeMapFrame from '../../components/visualization/ChangeMapFrame.svelte';
  import { changeMapOpen, restoreMapTrigger, setChangeMap } from '../../lib/navigation/change-map';
  import { eventDetails } from '../../lib/stores/eventDetails.svelte';
  import { timeline, type TimelineSnapshot } from '../../lib/stores/timeline.svelte';
  import { backendAvailable } from '../../lib/tauri/client';
  import type { AsyncState } from '../../lib/types/async-state';
  import type { ChangeMapRequest } from '../../lib/tauri/bindings';
  import TimelineToolbar from './TimelineToolbar.svelte';

  interface Props { data?: AsyncState<TimelineSnapshot>; }
  let { data }: Props = $props();
  const screenState = $derived(data ?? timeline.state);
  let mapOpen = $state(changeMapOpen());
  let queryRestored = $state(false);
  $effect.pre(() => {
    if (queryRestored) return;
    if (data === undefined) timeline.restoreFromHash();
    queryRestored = true;
  });
  const mapRequest = $derived.by(() => {
    const query = timeline.query;
    return {
      kind: 'window', range: query.range, categories: query.categories,
      importance_min: query.importance_min, entity_id: query.entity_id,
    } satisfies ChangeMapRequest;
  });

  onMount(() => {
    const syncMap = () => {
      const wasOpen = mapOpen;
      mapOpen = changeMapOpen();
      if (wasOpen && !mapOpen) restoreMapTrigger('[data-change-map-trigger="timeline"]');
    };
    window.addEventListener('hashchange', syncMap);
    const focusEvent = (message: Event) => {
      const event = (message as CustomEvent<{ event_id?: number }>).detail;
      if (typeof event?.event_id === 'number') void eventDetails.show(event.event_id);
    };
    window.addEventListener('what-changed:focus-event', focusEvent);
    if (data !== undefined || !backendAvailable()) {
      return () => {
        window.removeEventListener('hashchange', syncMap);
        window.removeEventListener('what-changed:focus-event', focusEvent);
      };
    }
    void timeline.load();
    let mounted = true;
    const pending = timeline.attachLive();
    void pending.then((unlisten) => { if (!mounted) unlisten(); });
    return () => {
      mounted = false;
      void pending.then((unlisten) => unlisten());
      window.removeEventListener('hashchange', syncMap);
      window.removeEventListener('what-changed:focus-event', focusEvent);
    };
  });

  function closeMap(): void {
    setChangeMap(false);
  }
</script>

{#if mapOpen && queryRestored}
  <ChangeMapFrame
    request={mapRequest}
    subtitle={`Current ${timeline.range.replaceAll('_', ' ')} range · ${timeline.filter} filter`}
    onclose={closeMap}
  />
{:else}
<ScreenFrame title="Timeline" flush>
  <TimelineToolbar
    range={timeline.range}
    filter={timeline.filter}
    customFrom={timeline.customFrom}
    customTo={timeline.customTo}
    onrange={(range) => { if (data === undefined) void timeline.setRange(range); }}
    onfilter={(filter) => { if (data === undefined) void timeline.applyFilter(filter); }}
    oncustom={(from, to) => { if (data === undefined) void timeline.setCustomRange(from, to); }}
    onmap={() => setChangeMap(true)}
  />

  <StateShell
    state={screenState}
    emptyTitle="No changes match these filters."
    emptyDescription="Try another range or clear a filter."
    emptyIcon="timeline"
    loadingRows={8}
    class="flex min-h-0 flex-1 flex-col"
  >
    {#snippet children(snapshot)}
      <TimelineList
        {snapshot}
        newCount={data === undefined ? timeline.newCount : 0}
        entering={timeline.entering}
        expanded={timeline.expandedKeys}
        onopen={(event, opener) => void eventDetails.show(event.id, opener)}
        onexpand={(cluster) => { if (data === undefined) void timeline.expand(cluster.cluster); }}
        onmore={() => { if (data === undefined) void timeline.loadMore(); }}
        onnewer={() => { if (data === undefined) void timeline.loadNewer(); }}
        onattop={(atTop) => timeline.setAtTop(atTop)}
        onshownew={() => { if (data === undefined) void timeline.showNew(); }}
      />
    {/snippet}
  </StateShell>
</ScreenFrame>
{/if}
