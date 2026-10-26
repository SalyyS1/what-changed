<script lang="ts">
  import type { Snippet } from 'svelte';

  /**
   * The one layout every screen uses: a fixed header row with the title and
   * optional actions, then a scrolling body. Screens differ in what they put
   * inside, not in how they are framed — which is what keeps the seven
   * destinations reading as one product.
   */
  interface Props {
    title: string;
    subtitle?: string;
    actions?: Snippet;
    children: Snippet;
    /** Remove body padding for screens that manage their own (timeline, map). */
    flush?: boolean;
  }

  let { title, subtitle, actions, children, flush = false }: Props = $props();
</script>

<div class="flex min-h-0 flex-1 flex-col" data-testid="screen">
  <header class="flex shrink-0 items-start justify-between gap-4 px-6 pt-5 pb-3">
    <div class="min-w-0">
      <h1 class="truncate font-display text-xl leading-tight font-medium text-text-primary">
        {title}
      </h1>
      {#if subtitle}
        <p class="mt-1 text-sm text-text-secondary">{subtitle}</p>
      {/if}
    </div>
    {#if actions}
      <div class="flex shrink-0 items-center gap-2">{@render actions()}</div>
    {/if}
  </header>
  <div class="flex min-h-0 flex-1 flex-col overflow-y-auto {flush ? '' : 'px-6 pb-6'}">
    {@render children()}
  </div>
</div>
