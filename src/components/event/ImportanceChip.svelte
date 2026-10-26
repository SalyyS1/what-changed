<script lang="ts">
  import type { Importance } from '../../lib/tauri/bindings';
  import Badge from '../ui/Badge.svelte';
  interface Props { importance: Importance; }
  let { importance }: Props = $props();
  const tone = $derived(importance === 'critical' ? 'danger' : importance === 'important' ? 'warning' : importance === 'notable' ? 'info' : 'neutral');
</script>
<Badge {tone}>{importance}</Badge>
