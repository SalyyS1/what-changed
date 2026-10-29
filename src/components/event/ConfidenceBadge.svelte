<script lang="ts">
  import type { Confidence } from '../../lib/tauri/bindings';
  import Badge from '../ui/Badge.svelte';
  interface Props { confidence: Confidence; }
  let { confidence }: Props = $props();
  const tone = $derived(confidence === 'high' ? 'success' : confidence === 'medium' ? 'info' : 'neutral');
</script>
<Badge {tone}>{confidence} confidence</Badge>
