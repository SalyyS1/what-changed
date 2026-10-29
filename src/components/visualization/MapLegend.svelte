<script lang="ts">
  const lanes = [
    'process', 'port', 'config', 'service', 'application',
    'update', 'driver', 'network', 'startup', 'system',
  ] as const;
  let open = $state(false);
</script>

<div class="absolute right-3 bottom-3 z-10">
  <button
    type="button"
    class="flex min-h-11 min-w-11 items-center justify-center rounded-md border border-border-subtle bg-surface-1/95 px-3 text-sm text-text-secondary shadow-md"
    aria-expanded={open}
    onclick={() => (open = !open)}
  >Legend</button>
  {#if open}
    <div class="absolute right-0 bottom-12 w-64 rounded-md border border-border-subtle bg-surface-1 p-3 shadow-lg" data-testid="map-legend">
      <p class="mb-2 text-caps text-text-muted">Categories</p>
      <ul class="grid grid-cols-2 gap-x-3 gap-y-1.5 text-xs text-text-secondary">
        {#each lanes as lane}
          <li class="flex items-center gap-2 capitalize">
            <span class="h-2.5 w-2.5 rounded-full" style:background={`var(--map-category-${lane})`} aria-hidden="true"></span>{lane}
          </li>
        {/each}
      </ul>
      <div class="mt-3 border-t border-border-subtle pt-2 text-xs text-text-secondary">
        <p><span class="mr-2 inline-block w-6 border-t-2 border-accent"></span>Recorded relationship</p>
        <p class="mt-1"><span class="mr-2 inline-block w-6 border-t border-dashed border-text-muted"></span>Close in time</p>
      </div>
    </div>
  {/if}
</div>
