<script lang="ts">
  import Skeleton from '../ui/Skeleton.svelte';

  /**
   * Content-shaped loading placeholder: `rows` skeleton lines of varying
   * width so the eye reads "a list is coming", not "the app is stuck".
   * Announced once via `aria-busy` on the region; the skeletons themselves
   * are hidden from assistive tech.
   */
  interface Props {
    rows?: number | undefined;
    label?: string;
    class?: string;
  }

  let { rows = 4, label = 'Loading', class: className = '' }: Props = $props();

  const widths = ['72%', '58%', '84%', '46%', '66%', '77%'];
</script>

<div
  data-testid="loading-block"
  role="status"
  aria-busy="true"
  aria-label={label}
  class="flex flex-col gap-3 px-6 py-8 {className}"
>
  {#each Array.from({ length: rows }, (_, i) => i) as i (i)}
    <div class="flex items-center gap-3">
      <Skeleton width="14px" height="14px" rounded="full" />
      <Skeleton width={widths[i % widths.length]} height="12px" />
    </div>
  {/each}
</div>
