<script lang="ts">
  import Button from '../ui/Button.svelte';
  interface Props { count: number; onclick: () => void; }
  let { count, onclick }: Props = $props();
</script>

{#if count > 0}
  <div class="pointer-events-none absolute top-2 left-1/2 z-20 -translate-x-1/2">
    <Button class="pointer-events-auto rounded-full" variant="primary" size="sm" {onclick}>
      {count} new {count === 1 ? 'change' : 'changes'}
    </Button>
  </div>
{/if}
