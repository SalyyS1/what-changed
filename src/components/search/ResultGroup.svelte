<script lang="ts">
  import type { SearchGroup } from '../../lib/stores/search.svelte';
  import type { SearchResultDto } from '../../lib/tauri/bindings';
  import ResultRow from './ResultRow.svelte';
  interface Props {
    group: SearchGroup;
    startIndex: number;
    activeIndex: number;
    onselect: (result: SearchResultDto, opener: HTMLElement) => void;
  }
  let { group, startIndex, activeIndex, onselect }: Props = $props();
</script>

<section aria-labelledby={`search-group-${startIndex}`}>
  <h3 id={`search-group-${startIndex}`} class="px-3 pt-3 pb-1 text-caps text-text-muted">{group.name}</h3>
  {#each group.results as result, offset (`${result.kind}:${result.kind === 'event' ? result.event.id : result.checkpoint.id}`)}
    <ResultRow {result} index={startIndex + offset} selected={activeIndex === startIndex + offset} {onselect} />
  {/each}
</section>
