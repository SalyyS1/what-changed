<script lang="ts">
  import { onMount } from 'svelte';
  import { eventDetails } from '../../lib/stores/eventDetails.svelte';
  import { search as sharedSearch, type SearchStore } from '../../lib/stores/search.svelte';
  import { timeline } from '../../lib/stores/timeline.svelte';
  import type { SearchResultDto } from '../../lib/tauri/bindings';
  import Dialog from '../ui/Dialog.svelte';
  import Input from '../ui/Input.svelte';
  import KeyHint from '../ui/KeyHint.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import Button from '../ui/Button.svelte';
  import ResultGroup from './ResultGroup.svelte';

  interface Props { controller?: SearchStore; shortcut?: boolean; }
  let { controller = sharedSearch, shortcut = true }: Props = $props();
  let wasOpen = $state(false);
  let results: HTMLDivElement;
  let searchRegion: HTMLDivElement;
  const activeResult = $derived(controller.flatResults[controller.activeIndex]);

  onMount(() => {
    if (!shortcut) return;
    const keydown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        if (eventDetails.open) {
          eventDetails.returnFocus = null;
          eventDetails.close();
          requestAnimationFrame(() => controller.show());
        } else {
          controller.show();
        }
      }
    };
    window.addEventListener('keydown', keydown);
    return () => window.removeEventListener('keydown', keydown);
  });

  $effect(() => {
    if (wasOpen && !controller.open) controller.close();
    wasOpen = controller.open;
  });

  function focusSearchOnOpen(event: Event): void {
    // Bits' default focus picks the Close button because it precedes the
    // search field in DOM order. Claim the open-focus event for the field.
    event.preventDefault();
    requestAnimationFrame(() => searchRegion?.querySelector<HTMLInputElement>('input')?.focus());
  }

  $effect(() => {
    const index = controller.activeIndex;
    if (!controller.open || controller.flatResults.length === 0) return;
    requestAnimationFrame(() => {
      const active = results?.querySelector<HTMLElement>(`#search-result-${index}`);
      active?.scrollIntoView?.({ block: 'nearest' });
    });
  });

  function inputKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      controller.move(1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      controller.move(-1);
    } else if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      if (activeResult?.kind === 'event') {
        event.preventDefault();
        showInSystemMap(activeResult);
      }
    } else if (event.key === 'Enter') {
      const result = controller.flatResults[controller.activeIndex];
      if (result) void select(result, event.currentTarget as HTMLElement);
    }
  }

  async function select(result: SearchResultDto, _opener: HTMLElement): Promise<void> {
    controller.close();
    if (result.kind === 'checkpoint') {
      window.location.hash = `#/checkpoints?checkpoint=${result.checkpoint.id}`;
      return;
    }
    if (result.event.entity_type === 'port') {
      await timeline.filterToEntity(result.event.entity_id);
      return;
    }
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    await eventDetails.show(result.event.id);
  }

  function showInSystemMap(result: SearchResultDto): void {
    if (result.kind !== 'event') return;
    controller.close();
    const params = new URLSearchParams({
      focus: result.event.entity_id,
      type: result.event.entity_type,
      depth: '2',
    });
    window.location.hash = `#/system-map?${params.toString()}`;
  }
</script>

<Dialog bind:open={controller.open} title="Search changes" description="Apps, files, ports, services, drivers, Docker, checkpoints and events." width="lg" onOpenAutoFocus={focusSearchOnOpen}>
  <div bind:this={searchRegion} class="mb-2 flex items-center gap-2">
    <Input
      value={controller.query}
      label="Search history"
      placeholder="Search an app name, a port, or a file name"
      autocomplete="off"
      autofocus
      aria-controls="search-results"
      aria-activedescendant={controller.flatResults.length > 0 ? `search-result-${controller.activeIndex}` : undefined}
      oninput={(event) => controller.setQuery(event.currentTarget.value)}
      onkeydown={inputKeydown}
      class="h-10"
    />
    <KeyHint keys="Esc" />
  </div>

  <div bind:this={results} id="search-results" role="listbox" aria-label="Search results" class="max-h-96 overflow-y-auto">
    {#if controller.state.status === 'loading'}
      <div class="space-y-2 px-3 py-4" aria-label="Searching">
        <Skeleton height="34px" /><Skeleton height="34px" /><Skeleton height="34px" />
      </div>
    {:else if controller.searching}
      <p class="px-4 py-8 text-center text-sm text-text-muted" role="status">Searching…</p>
    {:else if controller.state.status === 'error'}
      <div role="alert" class="px-3 py-8 text-center">
        <p class="text-sm text-danger">Search could not be completed.</p>
        {#if controller.state.retry}
          <button class="mt-2 text-sm text-accent hover:underline" onclick={controller.state.retry}>Try again</button>
        {/if}
      </div>
    {:else if controller.state.status === 'ready'}
      {@const groups = controller.state.data}
      {#each groups as group, groupIndex (group.name)}
        {@const startIndex = groups.slice(0, groupIndex).reduce((total, candidate) => total + candidate.results.length, 0)}
        <ResultGroup {group} {startIndex} activeIndex={controller.activeIndex} onselect={select} />
      {/each}
    {:else if controller.query.trim() !== ''}
      <div class="px-4 py-10 text-center">
        <p class="text-sm text-text-primary">No matches.</p>
        <p class="mt-1 text-sm text-text-muted">Try an app name, a port, or a file name.</p>
      </div>
    {:else}
      <div class="px-4 py-8 text-center text-sm text-text-muted">Start typing to search local history.</div>
    {/if}
  </div>
  {#if activeResult?.kind === 'event'}
    <div class="mt-2 flex justify-end border-t border-border-subtle pt-2">
      <Button size="sm" onclick={() => showInSystemMap(activeResult)} aria-label={`Show ${activeResult.event.entity_display_name} in System Map`}>
        Show in System Map <span class="text-text-muted">Ctrl+Enter</span>
      </Button>
    </div>
  {/if}
</Dialog>
