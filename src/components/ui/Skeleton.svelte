<script lang="ts">
  /**
   * A loading placeholder shaped like the content it stands in for — the
   * six-state rule forbids a spinner over an empty page. Width and height
   * are passed so a list of these can mimic a list of rows.
   */
  interface Props {
    width?: string | undefined;
    height?: string | undefined;
    rounded?: 'sm' | 'md' | 'full';
    class?: string;
  }
  let { width = '100%', height = '14px', rounded = 'sm', class: className = '' }: Props = $props();
  const radii = { sm: 'rounded-sm', md: 'rounded-md', full: 'rounded-full' };
</script>

<div
  aria-hidden="true"
  data-testid="skeleton"
  class="wc-skeleton {radii[rounded]} {className}"
  style="width:{width};height:{height}"
></div>

<style>
  .wc-skeleton {
    background: linear-gradient(
      90deg,
      var(--surface-2) 25%,
      var(--surface-3) 50%,
      var(--surface-2) 75%
    );
    background-size: 200% 100%;
    animation: wc-shimmer var(--t-loop-shimmer) linear infinite;
  }
</style>
