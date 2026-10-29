<script lang="ts">
  import type { GlobalHealth } from '../../lib/stores/home.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  interface Props { health: GlobalHealth; headline: string; }
  let { health, headline }: Props = $props();
  const label = $derived(health === 'recovery' ? 'Recovery needed' : health === 'degraded' ? 'Degraded' : health === 'paused' ? 'Paused' : 'Active');
  const tone = $derived(health === 'recovery' ? 'danger' : health === 'degraded' ? 'warning' : health === 'paused' ? 'warning' : 'success');
</script>

<div class="mb-4">
  <Badge {tone} dot>{label}</Badge>
  <h2 class="mt-3 font-display text-lg text-text-primary">{headline}</h2>
</div>
