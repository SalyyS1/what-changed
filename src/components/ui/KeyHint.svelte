<script lang="ts">
  /** A keyboard shortcut rendered as keycaps: `keys="Ctrl+K"`. */
  interface Props {
    keys: string;
    class?: string;
  }
  let { keys, class: className = '' }: Props = $props();
  const parts = $derived(keys.split('+'));
</script>

<span class="inline-flex items-center gap-0.5 {className}" aria-label={`Shortcut ${keys}`}>
  {#each parts as key, i (i)}
    <kbd
      class="inline-flex h-5 min-w-5 items-center justify-center rounded-sm border border-border-subtle bg-surface-2 px-1 font-mono text-xs text-text-muted"
      >{key}</kbd
    >
  {/each}
</span>
