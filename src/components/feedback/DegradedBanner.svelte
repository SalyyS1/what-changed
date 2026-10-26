<script lang="ts">
  import Button from '../ui/Button.svelte';
  import { collectors, isCollectorDegraded } from '../../lib/stores/collectors.svelte';
  import { router } from '../../lib/stores/router.svelte';
  import { routeById } from '../../app/routes';

  let dismissed = $state(false);
  const degraded = $derived(collectors.rows.filter(isCollectorDegraded));

  function openDiagnostics(): void {
    router.navigate(routeById('diagnostics')!);
  }
</script>

{#if !dismissed && degraded.length > 0}
  <div class="flex items-center justify-between gap-3 border-b border-warning/30 bg-warning/10 px-4 py-2 text-sm text-text-primary" role="status" aria-live="polite">
    <p class="m-0 min-w-0">Some things aren't being watched right now. <button class="underline underline-offset-2" onclick={openDiagnostics}>Open Diagnostics</button></p>
    <Button variant="ghost" size="sm" aria-label="Dismiss degraded mode notice" onclick={() => dismissed = true}>Dismiss</Button>
  </div>
{/if}
