<script lang="ts">
  /**
   * Determinate progress as a ring, `value` in [0, 1]. Indeterminate when
   * `value` is undefined — reserved for operations of genuinely unknown
   * length. Loading screens use Skeleton, never this.
   */
  interface Props {
    value?: number;
    size?: number;
    label: string;
    class?: string;
  }
  let { value, size = 20, label, class: className = '' }: Props = $props();

  const stroke = 2.5;
  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
  const clamped = $derived(value === undefined ? 0.25 : Math.min(1, Math.max(0, value)));
  const dash = $derived(c * clamped);
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 {size} {size}"
  role="progressbar"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={value === undefined ? undefined : Math.round(clamped * 100)}
  class="text-accent {value === undefined ? 'wc-ring-spin' : ''} {className}"
>
  <circle
    cx={size / 2}
    cy={size / 2}
    {r}
    fill="none"
    class="stroke-border-subtle"
    stroke-width={stroke}
  />
  <circle
    cx={size / 2}
    cy={size / 2}
    {r}
    fill="none"
    stroke="currentColor"
    stroke-width={stroke}
    stroke-linecap="round"
    stroke-dasharray="{dash} {c - dash}"
    transform="rotate(-90 {size / 2} {size / 2})"
    class="motion-base transition-[stroke-dasharray]"
  />
</svg>

<style>
  .wc-ring-spin {
    animation: wc-ring-spin var(--t-loop-spin) linear infinite;
  }
  @keyframes wc-ring-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
