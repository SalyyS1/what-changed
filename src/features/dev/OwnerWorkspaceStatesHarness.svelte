<script lang="ts">
  import Chip from '../../components/ui/Chip.svelte';
  import OwnerWorkspaceFixture from '../owner-workspace/OwnerWorkspaceFixture.svelte';
  let screen = $state<'apps' | 'overview' | 'change'>('overview');
  let fixtureState = $state<'loading' | 'empty' | 'error' | 'ready'>('ready');
  const screens = ['apps', 'overview', 'change'] as const;
  const states = ['loading', 'empty', 'error', 'ready'] as const;
</script>

<section class="mb-8" aria-labelledby="owner-harness-title">
  <h2 id="owner-harness-title" class="mb-2 text-caps text-text-muted">Owner workspace · isolated synthetic context · 900×600</h2>
  <div class="mb-3 flex flex-wrap gap-2">{#each screens as item (item)}<Chip selected={screen === item} onclick={() => (screen = item)}>{item}</Chip>{/each}{#each states as item (item)}<Chip selected={fixtureState === item} onclick={() => (fixtureState = item)}>{item}</Chip>{/each}</div>
  <div class="@container/shell flex h-150 w-225 max-w-full flex-col overflow-hidden rounded-lg border border-border-active bg-background">
    {#key `${screen}:${fixtureState}`}<OwnerWorkspaceFixture {screen} state={fixtureState} />{/key}
  </div>
</section>
