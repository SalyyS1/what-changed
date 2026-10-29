<script lang="ts">
  import type { DashboardCountsDto } from '../../lib/tauri/bindings';
  interface Props { counts: DashboardCountsDto; }
  let { counts }: Props = $props();
</script>

<p class="text-sm text-text-secondary font-tabular" aria-label={`${counts.total} changes, ${counts.important} important, ${counts.critical} critical`}>
  {counts.total} changes · {counts.important} important · {counts.critical} critical
</p>
