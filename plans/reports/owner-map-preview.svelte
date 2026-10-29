<script lang="ts">
  import OwnerMapFrame from '../../src/components/owner-map/OwnerMapFrame.svelte';
  import { ownerGraphFixture } from '../../src/lib/owner-graph/test-fixtures';
  const fixture = ownerGraphFixture();
  const before = structuredClone(fixture.reviews[0]!);
  before.run.id = 'synthetic-before'; before.run.started_at = 0;
  before.run.manifest.candidate.identity = 'Synthetic earlier candidate';
  const input = { ...fixture, reviews: [...fixture.reviews, before] };
</script>

<div class="mx-auto max-w-6xl p-4"><p class="mb-4 rounded-md border border-border-subtle p-3 text-sm text-text-secondary">Synthetic booking evidence · isolated QA harness · no production workspace or preview contacted</p><OwnerMapFrame {input} /></div>
