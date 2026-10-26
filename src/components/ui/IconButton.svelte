<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IconName } from './ui-types';

  interface Props extends HTMLButtonAttributes {
    icon: IconName;
    /** Required: an icon-only control must name itself. */
    label: string;
    size?: 'sm' | 'md';
    tone?: 'neutral' | 'danger';
  }

  let {
    icon,
    label,
    size = 'md',
    tone = 'neutral',
    type = 'button',
    class: className = '',
    ...rest
  }: Props = $props();

  const box = $derived(size === 'sm' ? 'h-7 w-7' : 'h-8 w-8');
  const iconSize = $derived(size === 'sm' ? 14 : 16);
  const tones = {
    neutral: 'text-text-secondary hover:bg-surface-2 hover:text-text-primary',
    danger: 'text-text-secondary hover:bg-danger hover:text-accent-contrast',
  };
</script>

<button
  {type}
  aria-label={label}
  title={label}
  data-testid="icon-button"
  class="inline-flex items-center justify-center rounded-md motion-fast transition-colors disabled:pointer-events-none disabled:opacity-45 {box} {tones[tone]} {className}"
  {...rest}
>
  <Icon name={icon} size={iconSize} />
</button>
