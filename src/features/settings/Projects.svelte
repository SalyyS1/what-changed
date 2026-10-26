<script lang="ts">
  import { onMount, tick } from 'svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Separator from '../../components/ui/Separator.svelte';
  import { formatAbsoluteTime } from '../../lib/formatting/time';
  import {
    PROJECT_LIMIT,
    ProjectsStore,
    projects,
  } from '../../lib/stores/projects.svelte';

  interface Props {
    active?: boolean;
    model?: ProjectsStore;
  }

  let { active = true, model = projects }: Props = $props();
  let root: HTMLElement;

  onMount(() => {
    if (active) void model.load();
  });

  async function removeProject(projectId: string): Promise<void> {
    const removedIndex = model.rows.findIndex((row) => row.id === projectId);
    if (!(await model.remove(projectId))) return;
    await tick();
    const remaining = Array.from(root.querySelectorAll<HTMLButtonElement>('[data-project-remove]'));
    const next = remaining[Math.min(removedIndex, remaining.length - 1)]
      ?? root.querySelector<HTMLButtonElement>('[data-add-project]');
    next?.focus();
  }
</script>

<div bind:this={root}>
  <Card level={1} class="mb-3">
    <div class="flex flex-col gap-4 @[680px]/shell:flex-row @[680px]/shell:items-start @[680px]/shell:justify-between">
      <div class="min-w-0">
        <p class="text-base text-text-primary">Create a workspace around a project root.</p>
        <p id="projects-privacy" class="mt-1 text-sm text-text-secondary">
          We use the folder boundary and branch name, never secret values from your environment.
        </p>
        {#if model.loaded}
          <p class="mt-2 text-xs text-text-muted font-tabular">{model.rows.length} of {PROJECT_LIMIT} workspaces</p>
        {/if}
      </div>
      <Button
        variant="primary"
        disabled={!active || !model.loaded || model.adding || model.atLimit}
        aria-describedby="projects-privacy projects-limit"
        data-add-project
        onclick={() => void model.chooseAndAdd()}
      >
        {model.adding ? 'Adding…' : 'Add workspace'}
      </Button>
    </div>
    <p id="projects-limit" class="mt-2 text-xs {model.atLimit ? 'text-warning' : 'sr-only'}">
      {model.atLimit ? 'The 20-workspace limit has been reached.' : 'You can register up to 20 workspaces.'}
    </p>
    {#if model.error}<p class="mt-3 rounded-md border border-danger/30 bg-danger/12 px-3 py-2 text-sm text-danger" role="alert">{model.error}</p>{/if}
  </Card>

  <StateShell
    state={model.state}
    emptyTitle="No workspaces yet."
    emptyDescription="Add a project root, then mark it working before you investigate a failure."
    emptyIcon="settings"
    errorTitle="Workspaces could not be loaded"
    loadingRows={2}
  >
    {#snippet children(rows)}
      <Card level={1} padded={false}>
        {#each rows as project, index (project.id)}
          {#if index > 0}<Separator />{/if}
          <article class="flex flex-col gap-3 px-4 py-4" aria-labelledby={`project-${project.id}`}>
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <h3 id={`project-${project.id}`} class="text-sm font-medium text-text-primary">{project.name}</h3>
                <Badge tone={project.baseline_checkpoint_id === null ? 'neutral' : 'success'} dot>
                  {project.baseline_checkpoint_id === null ? 'Needs baseline' : 'Working state saved'}
                </Badge>
                {#if project.branch}<Badge tone="neutral">{project.branch}</Badge>{/if}
              </div>
              <p class="selectable mt-1 break-all font-mono text-sm text-text-secondary" title={project.path}>{project.path}</p>
              {#if project.baseline_checkpoint_id !== null}
                <p class="mt-2 text-xs text-text-muted">
                  Baseline: {project.baseline_checkpoint_name ?? 'Working state'}
                  {#if project.baseline_checkpoint_created_at !== null} · {formatAbsoluteTime(project.baseline_checkpoint_created_at)}{/if}
                </p>
              {:else}
                <p class="mt-2 text-xs text-text-muted">Mark the project working before the next failure.</p>
              {/if}
            </div>
            <div class="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="primary"
                disabled={!active || model.marking.has(project.id)}
                onclick={() => void model.markWorking(project.id)}
              >{model.marking.has(project.id) ? 'Capturing…' : 'Mark working'}</Button>
              {#if project.baseline_checkpoint_id !== null}
                <Button size="sm" variant="secondary" disabled={!active} onclick={() => void model.clearBaseline(project.id)}>Clear baseline</Button>
              {/if}
              <Button
                size="sm"
                variant="danger"
                disabled={!active || model.removing.has(project.id)}
                aria-label={`Remove workspace ${project.name}`}
                data-project-remove
                onclick={() => void removeProject(project.id)}
              >{model.removing.has(project.id) ? 'Removing…' : 'Remove'}</Button>
            </div>
          </article>
        {/each}
      </Card>
    {/snippet}
  </StateShell>
</div>
