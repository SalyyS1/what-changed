<script lang="ts">
  import type { EntityKindAvailability, EntityType } from '../../lib/tauri/bindings';
  import { entityKindLabel, systemMapKinds } from '../../features/system-map/copy';
  import Chip from '../ui/Chip.svelte';

  interface Props {
    availability: EntityKindAvailability[];
    selected: EntityType[];
    ontoggle: (kind: EntityType) => void;
  }
  let { availability, selected, ontoggle }: Props = $props();
  const byKind = $derived(new Map(availability.map((item) => [item.kind, item])));
</script>

<div class="flex flex-wrap items-center gap-1.5" aria-label="Entity kinds">
  {#each systemMapKinds as kind (kind)}
    {@const status = byKind.get(kind)}
    {#if status && !status.available}
      <Chip
        selected={selected.length === 0 || selected.includes(kind)}
        disabledReason={status.reason ?? `No ${entityKindLabel[kind].toLocaleLowerCase()} recorded`}
        aria-label={`${entityKindLabel[kind]}, ${status.count} recorded`}
        onclick={() => ontoggle(kind)}
      >
        {entityKindLabel[kind]} · {status.count}
      </Chip>
    {:else}
      <Chip selected={selected.length === 0 || selected.includes(kind)} aria-label={`${entityKindLabel[kind]}${status ? `, ${status.count} recorded` : ''}`} onclick={() => ontoggle(kind)}>
        {entityKindLabel[kind]}{status ? ` · ${status.count}` : ''}
      </Chip>
    {/if}
  {/each}
</div>
