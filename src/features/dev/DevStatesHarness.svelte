<script lang="ts">
  import { navRoutes } from '../../app/routes';
  import { navScreens } from '../../app/screens';
  import Chip from '../../components/ui/Chip.svelte';
  import { settings } from '../../lib/stores/settings.svelte';
  import { fixtureFor, harnessStates, type HarnessRoute, type HarnessState } from './dev-states-fixtures';
  import DevSimulationPanel from './DevSimulationPanel.svelte';
  import ChangeMapStatesHarness from './ChangeMapStatesHarness.svelte';
  import SearchStatesHarness from './SearchStatesHarness.svelte';
  import SystemMapStatesHarness from './SystemMapStatesHarness.svelte';

  /**
   * `#/dev/states`: every screen in every shell state, in one scrollable
   * page, plus one cell rendered inside a 900×600 frame — the smallest
   * window the app allows. Together with `large` that is all six states
   * the product rule asks to be inspectable in one place. Dev builds only
   * (route table filters it out and the import is behind
   * `import.meta.env.DEV` in App.svelte).
   *
   * Frames are plain divs with `@container`, not iframes: an iframe would
   * need its own document, styles and theme bootstrap, and the container
   * query is what the real shell measures anyway.
   */
  const screens = navRoutes.filter((r): r is typeof r & { id: HarnessRoute } => r.id in navScreens);

  let smallRoute = $state<HarnessRoute>('home');
  let smallState = $state<HarnessState>('ready');
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-y-auto px-6 py-5" data-testid="dev-states">
  <h1 class="font-display text-xl text-text-primary">States harness</h1>
  <p class="mb-5 text-sm text-text-secondary">
    {screens.length} screens × {harnessStates.length} states. Dev builds only.
  </p>

  <DevSimulationPanel developerMode={settings.current?.developer_mode ?? false} />
  <SearchStatesHarness />
  <ChangeMapStatesHarness />
  <SystemMapStatesHarness />

  <section class="mb-8">
    <h2 class="mb-2 text-caps text-text-muted">Small window · 900×600</h2>
    <div class="mb-3 flex flex-wrap items-center gap-1.5">
      {#each screens as r (r.id)}
        <Chip selected={smallRoute === r.id} onclick={() => (smallRoute = r.id)}>{r.label}</Chip>
      {/each}
      <span class="mx-1 h-4 w-px bg-border-subtle" aria-hidden="true"></span>
      {#each harnessStates as s (s)}
        <Chip selected={smallState === s} onclick={() => (smallState = s)}>{s}</Chip>
      {/each}
    </div>
    <div
      class="@container/shell flex h-150 w-225 max-w-full flex-col overflow-hidden rounded-lg border border-border-active bg-background shadow-md"
    >
      {#key `${smallRoute}:${smallState}`}
        {@const Screen = navScreens[smallRoute]}
        <Screen data={fixtureFor(smallRoute, smallState)} />
      {/key}
    </div>
  </section>

  {#each screens as r (r.id)}
    {@const Screen = navScreens[r.id]}
    <section class="mb-8">
      <h2 class="mb-2 text-caps text-text-muted">{r.label}</h2>
      <div class="grid grid-cols-1 gap-4 @[1200px]/shell:grid-cols-2">
        {#each harnessStates as s (s)}
          <div class="flex flex-col gap-1">
            <span class="text-xs text-text-muted">{s}</span>
            <div
              class="@container/shell flex h-90 flex-col overflow-hidden rounded-lg border border-border-subtle bg-background"
            >
              <Screen data={fixtureFor(r.id, s)} />
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/each}
</div>
