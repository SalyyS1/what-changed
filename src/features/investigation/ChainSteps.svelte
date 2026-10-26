<script lang="ts">
  import { formatAbsoluteTime } from '../../lib/formatting/time';
  import type { ChainStepView } from './investigation-types';

  interface Props { steps: ChainStepView[]; }
  let { steps }: Props = $props();
</script>

{#if steps.length > 0}
  <div class="mt-3 border-l-2 border-accent/35 pl-3" data-testid="chain-steps">
    <p class="mb-2 text-caps text-text-muted">Change sequence</p>
    <ol class="m-0 flex list-none flex-col gap-2 p-0">
      {#each steps as step, index (step.id)}
        <li class="relative text-sm text-text-secondary">
          <span class="absolute -left-[1.05rem] top-1.5 h-1.5 w-1.5 rounded-full bg-accent" aria-hidden="true"></span>
          <span class="font-tabular text-text-muted">{index + 1}.</span>
          {step.title}
          {#if step.timestamp !== null}
            <span class="ml-1 text-xs text-text-muted">{formatAbsoluteTime(step.timestamp)}</span>
          {/if}
        </li>
      {/each}
    </ol>
  </div>
{/if}
