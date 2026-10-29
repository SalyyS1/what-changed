<script lang="ts">
  /**
   * Inline SVG icon set. Local paths, no icon font, no network. Stroke-based
   * so one `currentColor` serves every theme, and `aria-hidden` by default —
   * a bare icon is decoration; a control names itself with `aria-label`.
   */
  import type { IconName } from './ui-types';

  interface Props {
    name: IconName;
    size?: number;
    class?: string;
    label?: string;
  }

  let { name, size = 16, class: className = '', label }: Props = $props();

  const paths: Record<IconName, string> = {
    home: 'M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z',
    timeline: 'M4 6h16M4 12h10M4 18h13M18 12h2',
    investigate: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zm5 12 4 4',
    checkpoint: 'M6 3v18M6 4h11l-2 4 2 4H6',
    map: 'M6 5a3 3 0 1 0 0 6 3 3 0 0 0 0-6zm12 8a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM9 8h9v5M6 11v3a4 4 0 0 0 4 4h5',
    settings:
      'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm7.4-3 1.6-1-1.5-2.6-1.9.5a7 7 0 0 0-1.7-1l-.3-1.9h-3l-.3 1.9a7 7 0 0 0-1.7 1l-1.9-.5L7.2 11l1.6 1-1.6 1 1.5 2.6 1.9-.5a7 7 0 0 0 1.7 1l.3 1.9h3l.3-1.9a7 7 0 0 0 1.7-1l1.9.5 1.5-2.6z',
    diagnostics: 'M3 12h4l3-8 4 16 3-8h4',
    dev: 'm8 8-4 4 4 4m8-8 4 4-4 4m-2-10-4 12',
    minimize: 'M5 12h14',
    maximize: 'M5 5h14v14H5z',
    restore: 'M8 8h11v11H8zM5 15V5h10',
    close: 'M6 6l12 12M18 6 6 18',
    'chevron-down': 'm6 9 6 6 6-6',
    'chevron-right': 'm9 6 6 6-6 6',
    check: 'm5 12 5 5 9-10',
    search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zm5 12 4 4',
    x: 'M6 6l12 12M18 6 6 18',
    alert: 'M12 3 2 20h20zM12 9v5m0 3v.5',
    info: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zm0 8v5m0-8v.5',
    refresh: 'M20 12a8 8 0 1 1-2.3-5.7M20 4v5h-5',
    sun: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zm0-5v2m0 14v2M3 12h2m14 0h2M5.6 5.6l1.4 1.4m9.9 9.9 1.4 1.4m0-12.7-1.4 1.4M7 17l-1.4 1.4',
    moon: 'M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z',
    monitor: 'M4 5h16v11H4zM9 20h6m-3-4v4',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  class="shrink-0 {className}"
  aria-hidden={label ? undefined : 'true'}
  aria-label={label}
  role={label ? 'img' : undefined}
>
  <path d={paths[name]} />
</svg>
