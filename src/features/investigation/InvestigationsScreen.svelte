<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import ChangeMapFrame from '../../components/visualization/ChangeMapFrame.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import { changeMapOpen, mapInvestigationId, restoreMapTrigger, setChangeMap } from '../../lib/navigation/change-map';
  import { investigation } from '../../lib/stores/investigation.svelte';
  import { backendAvailable } from '../../lib/tauri/client';
  import type { AsyncState } from '../../lib/types/async-state';
  import InvestigationHistory from './InvestigationHistory.svelte';
  import type { InvestigationView, InvestigationWindow } from './investigation-types';
  import NoCauseState from './NoCauseState.svelte';
  import ProblemForm from './ProblemForm.svelte';
  import SuspectList from './SuspectList.svelte';
  import AiContextPackButton from './AiContextPackButton.svelte';
  import WorkspacePicker from '../workspace/WorkspacePicker.svelte';
  import { projects } from '../../lib/stores/projects.svelte';

  interface Props { data?: AsyncState<InvestigationView[]>; }
  let { data }: Props = $props();

  let problem = $state(investigation.draft);
  let window = $state<InvestigationWindow>(investigation.window);
  let fixtureSelectedId = $state<number | null>(null);
  let mapOpen = $state(changeMapOpen());
  let mapId = $state<number | null>(mapInvestigationId());

  const screenState = $derived(data ?? investigation.state);
  const selected = $derived.by(() => {
    if (data === undefined) return investigation.selected;
    if (screenState.status !== 'ready') return null;
    return screenState.data.find((row) => row.id === fixtureSelectedId) ?? screenState.data[0] ?? null;
  });
  const selectedWorkspace = $derived(
    selected?.projectId === null || selected?.projectId === undefined
      ? null
      : projects.rows.find((project) => project.id === selected.projectId) ?? null,
  );

  $effect(() => investigation.setDraft(problem));
  $effect(() => investigation.setWindow(window));

  onMount(() => {
    let detach = (): void => {};
    let disposed = false;
    const syncMap = () => {
      const wasOpen = mapOpen;
      const previousId = mapId;
      mapOpen = changeMapOpen();
      mapId = mapInvestigationId();
      if (data === undefined && mapOpen && mapId !== null) void investigation.select(mapId);
      if (wasOpen && !mapOpen && previousId !== null) {
        restoreMapTrigger(`[data-change-map-trigger="investigation-${previousId}"]`);
      }
    };
    globalThis.window.addEventListener('hashchange', syncMap);
    if (data === undefined && backendAvailable()) {
      void projects.load().then(() => {
        if (!disposed) investigation.setProjectId(projects.selectedId);
      });
      void investigation.load().then(() => {
        if (!disposed && mapOpen && mapId !== null) void investigation.select(mapId);
      });
      void investigation.attach().then((unlisten) => {
        if (disposed) unlisten();
        else detach = unlisten;
      });
    }
    return () => { disposed = true; detach(); globalThis.window.removeEventListener('hashchange', syncMap); };
  });

  function run(): void {
    investigation.setDraft(problem);
    investigation.setWindow(window);
    void investigation.start();
  }

  function selectWorkspace(projectId: string | null): void {
    investigation.setProjectId(projectId);
    const selectedProject = projects.rows.find((project) => project.id === projectId);
    const baselineId = selectedProject?.baseline_checkpoint_id;
    if (baselineId != null && window.kind === 'today') {
      window = { kind: 'since_checkpoint', id: baselineId };
    }
  }

  function select(id: number): void {
    if (data === undefined) void investigation.select(id);
    else fixtureSelectedId = id;
  }

  function openMap(id: number): void {
    setChangeMap(true, { id: String(id) });
  }

  function closeMap(): void {
    setChangeMap(false);
  }
</script>

{#if mapOpen && mapId !== null}
  <ChangeMapFrame
    request={{ kind: 'investigation', investigation_id: mapId }}
    subtitle={selected?.problem ?? 'Investigation evidence and chain shape.'}
    onclose={closeMap}
  />
{:else}
<ScreenFrame title="Investigations" subtitle="Describe the symptom in your own words.">
  <Card level={1} class="mb-4 shrink-0">
    <ProblemForm bind:problem bind:window running={data === undefined && investigation.running} onsubmit={run} />
    <div class="mt-3 flex flex-wrap items-center gap-2">
      <WorkspacePicker disabled={data !== undefined} onchange={selectWorkspace} />
      {#if projects.selected?.baseline_checkpoint_id === null && projects.selectedId !== null}
        <p class="text-xs text-text-muted">This workspace has no known-good baseline yet.</p>
      {/if}
    </div>
    {#if data === undefined && investigation.error}
      <p class="mt-2 text-sm text-danger" role="alert">{investigation.error}</p>
    {/if}
  </Card>

  <StateShell
    state={screenState}
    emptyTitle="No investigations yet."
    emptyDescription="Describe what stopped working and the engine will rank the changes that could explain it."
    emptyIcon="investigate"
    loadingRows={5}
    class="overflow-y-auto"
  >
    {#snippet children(rows)}
      <div class="grid min-h-0 grid-cols-1 gap-4 @[760px]/shell:grid-cols-[minmax(0,1fr)_15rem]">
        <section class="min-w-0" aria-label="Investigation results">
          {#if selected}
            <div class="mb-3 flex flex-wrap items-end justify-between gap-2">
              <div>
                <p class="text-caps text-text-muted">Investigation</p>
                <h2 class="font-display text-lg text-text-primary">{selected.problem}</h2>
                <p class="mt-1 text-xs text-text-muted">
                  Analysed {selected.totalEvents.toLocaleString()} changes in this window.
                </p>
                <p class="mt-1 text-xs text-text-muted" data-testid="result-workspace">
                  Result workspace: {#if selected.projectId === null}All recorded changes{:else if selectedWorkspace}{selectedWorkspace.name}{:else}Unavailable (workspace removed){/if}
                </p>
              </div>
              <div class="flex flex-wrap items-center justify-end gap-2">
                <AiContextPackButton investigationId={selected.id} />
                <Button
                  size="sm"
                  data-change-map-trigger={`investigation-${selected.id}`}
                  onclick={() => openMap(selected.id)}
                >View as map</Button>
              </div>
            </div>

            {#if selected.droppedEvents > 0}
              <p class="mb-3 rounded-md bg-info/10 px-3 py-2 text-sm text-info" data-testid="downsample-disclosure">
                Analysed the 5,000 most important changes in this window. {selected.droppedEvents.toLocaleString()} routine changes were left out.
              </p>
            {/if}

            {#if selected.suspects.length > 0}
              <SuspectList suspects={selected.suspects} />
            {:else}
              <NoCauseState reason={selected.noCauseReason ?? 'nothing_changed'} />
            {/if}
          {/if}
        </section>

        <aside class="min-w-0 border-t border-border-subtle pt-3 @[760px]/shell:border-l @[760px]/shell:border-t-0 @[760px]/shell:pl-4 @[760px]/shell:pt-0">
          <InvestigationHistory
            {rows}
            selectedId={selected?.id ?? null}
            onselect={select}
            ondelete={data === undefined ? (id) => void investigation.remove(id) : undefined}
          />
        </aside>
      </div>
    {/snippet}
  </StateShell>
</ScreenFrame>
{/if}
