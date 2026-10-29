<script lang="ts">
  import SearchPalette from '../../components/search/SearchPalette.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Chip from '../../components/ui/Chip.svelte';
  import { SearchStore, groupSearchResults } from '../../lib/stores/search.svelte';
  import { empty, failed, loading, ready } from '../../lib/types/async-state';
  import type { EntityType, SearchResultDto } from '../../lib/tauri/bindings';
  import { harnessStates, type HarnessState } from './dev-states-fixtures';

  const controller = new SearchStore({ search: async () => [] });
  let selected = $state<HarnessState>('ready');
  const now = Date.now();

  function result(id: number, entityType: EntityType = 'app'): SearchResultDto {
    const display = entityType === 'port' ? `Port ${3000 + id}` : `${entityType} ${id}`;
    return {
      kind: 'event',
      category: entityType === 'port' ? 'network' : entityType === 'driver' ? 'driver' : 'application',
      importance: id % 5 === 0 ? 'important' : 'notable',
      event: {
        id,
        timestamp: now - id * 60_000,
        category: entityType === 'port' ? 'network' : entityType === 'driver' ? 'driver' : 'application',
        event_type: entityType === 'port' ? 'opened' : 'updated',
        entity_type: entityType,
        entity_id: `${entityType}:${id}`,
        entity_display_name: display,
        severity: 'info',
        importance: id % 5 === 0 ? 'important' : 'notable',
        confidence: 'high',
        source_collector: 'synthetic',
        before: null,
        after: null,
        field: null,
        user_friendly_summary: `${display} changed`,
        occurred_at_is_uncertain: false,
      },
    };
  }

  const readyResults: SearchResultDto[] = [
    result(1, 'app'),
    result(2, 'file'),
    result(3, 'port'),
    result(4, 'service'),
    result(5, 'driver'),
    result(6, 'container'),
    { kind: 'checkpoint', checkpoint: { id: 7, name: 'Before audio driver update', created_at: now - 3_600_000 } },
    result(8, 'system'),
  ];

  function openPreview(): void {
    controller.show();
    controller.query = selected === 'empty' ? 'not-a-real-thing' : selected === 'loading' ? 'searching' : 'audio';
    controller.activeIndex = 0;
    controller.searching = false;
    switch (selected) {
      case 'loading': controller.state = loading(); break;
      case 'empty': controller.state = empty(); break;
      case 'error': controller.state = failed('Search is temporarily unavailable.', openPreview); break;
      case 'ready': controller.state = ready(groupSearchResults(readyResults)); break;
      case 'large': {
        const entityTypes: EntityType[] = ['app', 'file', 'port', 'service', 'driver', 'container', 'system'];
        controller.state = ready(groupSearchResults(Array.from({ length: 100 }, (_, index) => result(index + 1, entityTypes[index % entityTypes.length]))));
        break;
      }
    }
  }
</script>

<section class="mb-8 rounded-lg border border-border-subtle bg-surface-1 p-4" aria-labelledby="search-harness-heading">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h2 id="search-harness-heading" class="text-caps text-text-muted">Search palette</h2>
      <p class="mt-1 text-sm text-text-secondary">Overlay states at the current window size.</p>
    </div>
    <Button onclick={openPreview}>Open {selected} state</Button>
  </div>
  <div class="mt-3 flex flex-wrap gap-1.5" aria-label="Search state">
    {#each harnessStates as state (state)}
      <Chip selected={selected === state} onclick={() => (selected = state)}>{state}</Chip>
    {/each}
  </div>
</section>

<SearchPalette {controller} shortcut={false} />
