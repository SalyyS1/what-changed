<script lang="ts">
  import type { SuspectView } from './investigation-types';
  import SuspectCard from './SuspectCard.svelte';

  interface Props { suspects: SuspectView[]; }
  let { suspects }: Props = $props();
</script>

<ol class="m-0 grid list-none grid-cols-1 gap-3 p-0 @[860px]/shell:grid-cols-2">
  {#each suspects as suspect, index (suspect.event.id)}
    <li><SuspectCard {suspect} rank={index + 1} /></li>
  {/each}
</ol>
