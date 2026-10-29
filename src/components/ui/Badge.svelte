<script lang="ts">
  import type { Snippet } from 'svelte';

  type Tone = 'neutral' | 'accent' | 'success' | 'warning' | 'danger' | 'info';

  interface Props {
    tone?: Tone;
    /** Leading dot, for status ("● Active"). */
    dot?: boolean;
    class?: string;
    children: Snippet;
  }

  let { tone = 'neutral', dot = false, class: className = '', children }: Props = $props();

  const tones: Record<Tone, string> = {
    neutral: 'bg-surface-3 text-text-secondary',
    accent: 'bg-accent/15 text-accent',
    success: 'bg-success/15 text-success',
    warning: 'bg-warning/15 text-warning',
    danger: 'bg-danger/15 text-danger',
    info: 'bg-info/15 text-info',
  };
</script>

<span
  data-testid="badge"
  class="inline-flex h-5 items-center gap-1.5 rounded-sm px-1.5 text-xs font-medium whitespace-nowrap {tones[tone]} {className}"
>
  {#if dot}<span class="h-1.5 w-1.5 rounded-full bg-current" aria-hidden="true"></span>{/if}
  {@render children()}
</span>
