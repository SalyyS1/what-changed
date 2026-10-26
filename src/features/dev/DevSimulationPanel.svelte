<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Icon from '../../components/ui/Icon.svelte';
  import ProgressRing from '../../components/ui/ProgressRing.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Switch from '../../components/ui/Switch.svelte';
  import { seedSynthetic, simulateEvent } from '../../lib/tauri/client';

  interface Props {
    developerMode: boolean;
  }

  type Profile = 'day' | 'week' | 'burst' | 'demo' | '10k' | '100k' | '1m';
  type SimulationKind =
    | 'driver_update'
    | 'port_change'
    | 'process_crash'
    | 'dns_change'
    | 'docker_restart'
    | 'config_diff';
  type Operation =
    | { state: 'idle'; message: string }
    | { state: 'pending'; message: string }
    | { state: 'success'; message: string }
    | { state: 'error'; message: string };

  let { developerMode }: Props = $props();

  const profiles = [
    { value: 'day', label: 'One day' },
    { value: 'week', label: 'One week' },
    { value: 'burst', label: 'Burst' },
    { value: 'demo', label: 'Linked demo story (8 events)' },
    { value: '10k', label: '10,000 events' },
    { value: '100k', label: '100,000 events' },
    { value: '1m', label: '1 million events' },
  ] as const;

  const simulations: readonly { kind: SimulationKind; label: string }[] = [
    { kind: 'driver_update', label: 'Driver update' },
    { kind: 'port_change', label: 'Port change' },
    { kind: 'process_crash', label: 'Process crash' },
    { kind: 'dns_change', label: 'DNS change' },
    { kind: 'docker_restart', label: 'Docker restart' },
    { kind: 'config_diff', label: 'Config diff' },
  ];
  const countFormat = new Intl.NumberFormat('en-US');

  let profile = $state<Profile>('day');
  let relations = $state(false);
  let operation = $state<Operation>({
    state: 'idle',
    message: 'Choose a corpus or simulate one event.',
  });
  const pending = $derived(operation.state === 'pending');
  const includesRelations = $derived(profile === 'demo' || relations);

  function profileLabel(value: Profile): string {
    return profiles.find((candidate) => candidate.value === value)?.label ?? value;
  }

  async function seed(): Promise<void> {
    const selected = profile;
    const selectedRelations = selected === 'demo' || relations;
    operation = { state: 'pending', message: `Seeding ${profileLabel(selected).toLowerCase()}…` };
    try {
      const count = await seedSynthetic(selected, selectedRelations);
      operation = {
        state: 'success',
        message: `${countFormat.format(count)} synthetic events added${selectedRelations ? ' with relations' : ''}.`,
      };
    } catch {
      // The typed client logs unrecognised IPC failures in development. Keep
      // this surface intentionally generic so an internal value never lands
      // in a screenshot or accessibility announcement.
      operation = {
        state: 'error',
        message: 'Synthetic data could not be added. Check the logs and try again.',
      };
    }
  }

  async function simulate(kind: SimulationKind, label: string): Promise<void> {
    operation = { state: 'pending', message: `Simulating ${label.toLowerCase()}…` };
    try {
      await simulateEvent(kind);
      operation = { state: 'success', message: `${label} event added.` };
    } catch {
      operation = {
        state: 'error',
        message: 'The event could not be simulated. Check the logs and try again.',
      };
    }
  }
</script>

<section class="mb-8" aria-labelledby="simulation-panel-title" data-testid="dev-simulation-panel">
  <div class="mb-2 flex items-center gap-2">
    <h2 id="simulation-panel-title" class="text-caps text-text-muted">Event simulator</h2>
    <Badge tone="warning">Synthetic data</Badge>
  </div>

  <Card level={1}>
    {#if !developerMode}
      <div class="flex items-start gap-3" data-testid="developer-mode-required">
        <span
          class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-warning/15 text-warning"
          aria-hidden="true"
        >
          <Icon name="dev" size={16} />
        </span>
        <div>
          <p class="text-base font-medium text-text-primary">Developer mode is off</p>
          <p class="mt-0.5 text-sm text-text-secondary">
            Enable it in Settings before creating synthetic history.
          </p>
        </div>
      </div>
    {:else}
      <div class="grid gap-5 @[760px]/shell:grid-cols-[minmax(15rem,0.9fr)_minmax(20rem,1.5fr)]">
        <fieldset class="min-w-0">
          <legend class="mb-2 text-sm font-medium text-text-primary">Seed a corpus</legend>
          <div class="flex items-end gap-2">
            <div class="min-w-0 flex-1">
              <p class="mb-1 text-xs text-text-muted">Profile</p>
              <Select
                options={profiles}
                value={profile}
                label="Synthetic data profile"
                disabled={pending}
                onchange={(value) => (profile = value as Profile)}
              />
            </div>
            <Button variant="primary" size="md" disabled={pending} onclick={() => void seed()}>
              Seed profile
            </Button>
          </div>
          <div class="mt-3 flex items-center justify-between gap-4 rounded-md bg-surface-2 px-3 py-2">
            <div>
              <p class="text-sm text-text-primary">Include relations</p>
              <p class="text-xs text-text-muted">
                {profile === 'demo' ? 'Always included in the synthetic demo story' : '--relations'}
              </p>
            </div>
            <Switch
              checked={includesRelations}
              label="Include synthetic entity relations"
              disabled={pending || profile === 'demo'}
              onchange={(checked) => (relations = checked)}
            />
          </div>
          {#if profile === 'demo'}
            <p class="mt-2 text-xs text-text-muted">
              Fictional observations and historical edges for a walkthrough; no causal claim.
            </p>
          {/if}
        </fieldset>

        <fieldset class="min-w-0">
          <legend class="mb-2 text-sm font-medium text-text-primary">Simulate one event</legend>
          <div class="grid grid-cols-2 gap-2 @[520px]/shell:grid-cols-3">
            {#each simulations as event (event.kind)}
              <Button
                variant="secondary"
                size="sm"
                disabled={pending}
                class="justify-start"
                onclick={() => void simulate(event.kind, event.label)}
              >
                {event.label}
              </Button>
            {/each}
          </div>
        </fieldset>
      </div>

      <div
        class="mt-4 flex min-h-8 items-center gap-2 border-t border-border-subtle pt-3 text-sm {operation.state ===
        'error'
          ? 'text-danger'
          : operation.state === 'success'
            ? 'text-success'
            : 'text-text-secondary'}"
        role={operation.state === 'error' ? 'alert' : 'status'}
        aria-live="polite"
        data-state={operation.state}
      >
        {#if operation.state === 'pending'}
          <ProgressRing label="Synthetic data operation in progress" size={16} />
        {:else if operation.state === 'success'}
          <Icon name="check" size={16} />
        {:else if operation.state === 'error'}
          <Icon name="alert" size={16} />
        {:else}
          <Icon name="info" size={16} />
        {/if}
        <span>{operation.message}</span>
      </div>
    {/if}
  </Card>
</section>
