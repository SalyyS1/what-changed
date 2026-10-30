<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Chip from '../../components/ui/Chip.svelte';
  import Button from '../../components/ui/Button.svelte';
  import { home, type HomeSnapshot } from '../../lib/stores/home.svelte';
  import { backendAvailable } from '../../lib/tauri/client';
  import type { AsyncState } from '../../lib/types/async-state';
  import ImportantChanges from './ImportantChanges.svelte';
  import ProblemInput from './ProblemInput.svelte';
  import RecoverySurface from './RecoverySurface.svelte';
  import StatusHeadline from './StatusHeadline.svelte';
  import TodayCounts from './TodayCounts.svelte';
  import { onboardingCopy } from '../onboarding/copy';
  import WorkspacePicker from '../workspace/WorkspacePicker.svelte';
  import { projects } from '../../lib/stores/projects.svelte';
  import { investigation } from '../../lib/stores/investigation.svelte';
  import { router } from '../../lib/stores/router.svelte';
  import { routeById } from '../../app/routes';

  interface Props { data?: AsyncState<HomeSnapshot>; }
  let { data }: Props = $props();
  const state = $derived(data ?? home.state);
  const ranges = [
    { label: 'Today', range: { kind: 'today' } as const },
    { label: 'Since last restart', range: { kind: 'since_last_restart' } as const },
    { label: 'Yesterday', range: { kind: 'yesterday' } as const },
  ];

  onMount(() => {
    if (data === undefined && backendAvailable()) void home.load();
    if (data === undefined && backendAvailable()) {
      void projects.load().then(() => investigation.setProjectId(projects.selectedId));
    }
  });

  function selectWorkspace(projectId: string | null): void {
    investigation.setProjectId(projectId);
    const selectedProject = projects.rows.find((project) => project.id === projectId);
    const baselineId = selectedProject?.baseline_checkpoint_id;
    if (baselineId != null && investigation.window.kind === 'today') {
      investigation.setWindow({ kind: 'since_checkpoint', id: baselineId });
    }
  }
</script>

{#if state.status === 'ready' && state.data.health === 'recovery'}
  <RecoverySurface />
{:else}
  <ScreenFrame title="Home">
    <Card level={1} class="mb-4 border-accent/25 bg-accent/5">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="min-w-0">
          <p class="text-caps text-accent">Building with AI?</p>
          <h2 class="mt-1 text-base font-medium text-text-primary">Keep the intent with the app</h2>
          <p class="mt-1 max-w-2xl text-sm leading-relaxed text-text-secondary">
            Save what your app should do, the rules it must preserve, and the next change you want to make.
            Your workspace stays local and can become a reviewed Markdown brief for your coding AI.
          </p>
        </div>
        <Button variant="secondary" onclick={() => router.navigate(routeById('owner-apps')!)}>
          Open My apps
        </Button>
      </div>
    </Card>
    <Card level={1} class="mb-4">
      <ProblemInput />
      <div class="mt-3 flex flex-wrap items-center gap-2">
        <WorkspacePicker onchange={selectWorkspace} />
        <p class="text-xs text-text-muted">Scope the next investigation to recorded changes associated with this project.</p>
      </div>
      <div class="mt-3 flex flex-wrap items-center gap-1.5">
        {#each ranges as item (item.label)}
          <Chip
            selected={home.range.kind === item.range.kind}
            disabled={data !== undefined}
            onclick={() => void home.setRange(item.range)}
          >{item.label}</Chip>
        {/each}
      </div>
    </Card>

    <StateShell
      {state}
      emptyTitle={onboardingCopy.emptyHome}
      emptyIcon="check"
      loadingRows={3}
    >
      {#snippet children(snapshot)}
        <StatusHeadline health={snapshot.health} headline={snapshot.summary.status_line} />
        <TodayCounts counts={snapshot.summary.today} />
        <div class="mt-5"><ImportantChanges events={snapshot.summary.top_important} /></div>
      {/snippet}
    </StateShell>

    <p class="mt-auto pt-6 text-xs text-text-muted">Your history stays on this PC.</p>
  </ScreenFrame>
{/if}
