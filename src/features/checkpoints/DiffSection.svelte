<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import type { CheckpointDiffDto, DiffItemDto } from '../../lib/tauri/bindings';

  type DiffKind = 'added' | 'removed' | 'changed';
  type CategoryGroup = CheckpointDiffDto['grouped_by_category'][number];
  interface Props { title: 'Added' | 'Removed' | 'Changed'; kind: DiffKind; groups: CategoryGroup[]; technical: boolean; }
  let { title, kind, groups, technical }: Props = $props();
  const populatedGroups = $derived(groups.filter((group) => group[kind].length > 0));
  const itemCount = $derived(populatedGroups.reduce((total, group) => total + group[kind].length, 0));
  const tones = { Added: 'success', Removed: 'danger', Changed: 'warning' } as const;
</script>

<section aria-labelledby={`diff-${title.toLowerCase()}`}>
  <div class="mb-2 flex items-center gap-2">
    <h2 id={`diff-${title.toLowerCase()}`} class="text-caps text-text-muted">{title}</h2>
    <Badge tone={tones[title]}>{itemCount}</Badge>
  </div>
  {#if itemCount === 0}
    <p class="rounded-md border border-border-subtle bg-surface-1 px-3 py-4 text-sm text-text-muted">Nothing {title.toLowerCase()}.</p>
  {:else}
    <div class="flex flex-col gap-3">
      {#each populatedGroups as group (group.category)}
        {@const categoryItems = group[kind] as DiffItemDto[]}
        <div class="overflow-hidden rounded-md border border-border-subtle bg-surface-1">
          <h3 class="border-b border-border-subtle bg-surface-2 px-3 py-1.5 text-xs font-medium uppercase tracking-wide text-text-muted">{group.category} · {categoryItems.length}</h3>
          <ul class="m-0 list-none divide-y divide-border-subtle p-0">
            {#each categoryItems as item, index (`${item.entity_display_name}:${item.field}:${index}`)}
              <li class="px-3 py-2 text-sm">
                <p class="text-text-primary">{item.entity_display_name}</p>
                {#if technical}
                  {#if item.field}<p class="mt-1 font-mono text-xs text-text-muted">{item.field}</p>{/if}
                  {#if title === 'Changed'}
                    <p class="mt-1 break-all font-mono text-xs text-text-secondary"><span class="text-danger">{item.before ?? '—'}</span> → <span class="text-success">{item.after ?? '—'}</span></p>
                  {/if}
                {/if}
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  {/if}
</section>
