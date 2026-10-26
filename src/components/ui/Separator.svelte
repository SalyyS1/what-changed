<script lang="ts">
  interface Props {
    orientation?: 'horizontal' | 'vertical';
    class?: string;
  }
  let { orientation = 'horizontal', class: className = '' }: Props = $props();
</script>

<div
  role="separator"
  aria-orientation={orientation}
  data-testid="separator"
  class="shrink-0 bg-border-subtle {orientation === 'horizontal' ? 'h-px w-full' : 'h-full w-px'} {className}"
></div>
