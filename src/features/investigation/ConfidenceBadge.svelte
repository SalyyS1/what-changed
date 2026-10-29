<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import type { ConfidenceClass } from './investigation-types';

  interface Props { confidence: ConfidenceClass; }
  let { confidence }: Props = $props();

  const label: Record<ConfidenceClass, string> = {
    confirmed: 'Confirmed',
    likely: 'Likely',
    correlated: 'Correlated',
  };
  const tone: Record<ConfidenceClass, 'success' | 'accent' | 'neutral'> = {
    confirmed: 'success',
    likely: 'accent',
    correlated: 'neutral',
  };
</script>

<Badge tone={tone[confidence]}>{label[confidence]}</Badge>
