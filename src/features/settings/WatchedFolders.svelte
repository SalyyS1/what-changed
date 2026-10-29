<script lang="ts">
  import { onMount, tick } from 'svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Separator from '../../components/ui/Separator.svelte';
  import {
    WATCHED_LOCATION_LIMIT,
    WatchedLocationsStore,
    watchedLocations,
  } from '../../lib/stores/watched-locations.svelte';

  interface Props {
    /** False in the states harness and outside the Tauri window. */
    active?: boolean;
    /** Injection point for focused component tests. */
    model?: WatchedLocationsStore;
  }

  let { active = true, model = watchedLocations }: Props = $props();
  let root: HTMLElement;

  onMount(() => {
    if (active) void model.load();
  });

  async function removeLocation(locationId: number): Promise<void> {
    const removedIndex = model.rows.findIndex((row) => row.id === locationId);
    if (!(await model.remove(locationId))) return;

    // Removing the focused control must leave keyboard focus somewhere useful.
    await tick();
    const remaining = Array.from(
      root.querySelectorAll<HTMLButtonElement>('[data-watched-folder-remove]'),
    );
    const next = remaining[Math.min(removedIndex, remaining.length - 1)]
      ?? root.querySelector<HTMLButtonElement>('[data-add-watched-folder]');
    next?.focus();
  }
</script>

<div bind:this={root}>
  <Card level={1} class="mb-3">
    <div class="flex flex-col gap-4 @[680px]/shell:flex-row @[680px]/shell:items-start @[680px]/shell:justify-between">
      <div class="min-w-0">
        <p class="text-base text-text-primary">Choose the folders whose configuration changes matter.</p>
        <p id="watched-folders-privacy" class="mt-1 text-sm text-text-secondary">
          We record what changed, never the secret values.
        </p>
        {#if model.loaded}
          <p class="mt-2 text-xs text-text-muted font-tabular">
            {model.rows.length} of {WATCHED_LOCATION_LIMIT} folders
          </p>
        {/if}
      </div>
      <Button
        variant="primary"
        disabled={!active || !model.loaded || model.adding || model.atLimit}
        aria-describedby="watched-folders-privacy watched-folders-limit"
        data-add-watched-folder
        onclick={() => void model.chooseAndAdd()}
      >
        {model.adding ? 'Adding…' : 'Add folder'}
      </Button>
    </div>
    <p id="watched-folders-limit" class="mt-2 text-xs {model.atLimit ? 'text-warning' : 'sr-only'}">
      {model.atLimit ? 'The 20-folder limit has been reached.' : 'You can watch up to 20 folders.'}
    </p>
    {#if model.error}
      <p class="mt-3 rounded-md border border-danger/30 bg-danger/12 px-3 py-2 text-sm text-danger" role="alert">
        {model.error}
      </p>
    {/if}
  </Card>

  <StateShell
    state={model.state}
    emptyTitle="No folders are watched yet."
    emptyDescription="Use Add folder to choose a project or configuration folder."
    emptyIcon="settings"
    errorTitle="Watched folders could not be loaded"
    loadingRows={2}
  >
    {#snippet children(locations)}
      <Card level={1} padded={false}>
        {#each locations as location, index (location.id)}
          {#if index > 0}<Separator />{/if}
          <article class="flex flex-col gap-3 px-4 py-4 @[680px]/shell:flex-row @[680px]/shell:items-center @[680px]/shell:justify-between" aria-labelledby={`watched-folder-${location.id}`}>
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <h3 id={`watched-folder-${location.id}`} class="text-sm font-medium text-text-primary">
                  Watched folder
                </h3>
                <Badge tone={location.available && location.enabled ? 'success' : 'warning'} dot>
                  {location.available && location.enabled ? 'Available' : 'Unavailable'}
                </Badge>
              </div>
              <p class="selectable mt-1 break-all font-mono text-sm text-text-secondary" title={location.path}>
                {location.path}
              </p>
              {#if !location.available}
                <p class="mt-1 text-xs text-text-muted">This folder cannot be reached right now. Watching will resume when it returns.</p>
              {/if}
            </div>
            <Button
              variant="danger"
              size="sm"
              disabled={!active || model.removing.has(location.id)}
              aria-label={`Remove watched folder ${location.path}`}
              data-watched-folder-remove
              onclick={() => void removeLocation(location.id)}
            >
              {model.removing.has(location.id) ? 'Removing…' : 'Remove'}
            </Button>
          </article>
        {/each}
      </Card>
    {/snippet}
  </StateShell>
</div>
