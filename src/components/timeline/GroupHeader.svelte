<script lang="ts">
  interface Props { label: string; sticky?: boolean; }
  let { label, sticky = false }: Props = $props();
</script>

<div
  class="flex h-8 items-center border-y border-border-subtle bg-background/95 px-6 text-caps text-text-muted {sticky ? 'sticky top-0 z-10 shadow-sm' : ''}"
  style="content-visibility:auto"
  data-testid="timeline-group"
>
  {label}
</div>
