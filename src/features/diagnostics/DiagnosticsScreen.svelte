<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import { formatBytes } from '../../lib/formatting/bytes';
  import { formatAbsoluteTime } from '../../lib/formatting/time';
  import { diagnostics } from '../../lib/stores/diagnostics.svelte';
  import { exportDiagnostics } from '../../lib/tauri/client';
  import type { AsyncState } from '../../lib/types/async-state';
  import type {
    CollectorState,
    CollectorWakeTier,
    DiagnosticsDto,
    DockerCollectorState,
  } from '../../lib/tauri/bindings';
  import {
    collectorCadenceLabel,
    durationLabel,
    type CollectorTone,
  } from '../settings/collector-presentation';

  interface Props {
    /** Dev States supplies a snapshot; ordinary routing omits it and uses live IPC. */
    data?: AsyncState<DiagnosticsDto>;
  }

  let { data }: Props = $props();
  const viewState = $derived(data ?? diagnostics.state);
  const live = $derived(data === undefined);
  let exportState = $state<string | null>(null);

  const collectorState: Record<CollectorState, { label: string; tone: CollectorTone }> = {
    running: { label: 'Running', tone: 'success' },
    idle: { label: 'Idle', tone: 'info' },
    paused: { label: 'Paused', tone: 'warning' },
    failed: { label: 'Failed', tone: 'danger' },
    degraded: { label: 'Degraded', tone: 'warning' },
    disabled: { label: 'Disabled', tone: 'neutral' },
    unavailable: { label: 'Unavailable', tone: 'warning' },
  };

  const databaseState: Record<DiagnosticsDto['db']['state'], { label: string; tone: CollectorTone }> = {
    ready: { label: 'Ready', tone: 'success' },
    corrupt: { label: 'Corrupt', tone: 'danger' },
    schema_mismatch: { label: 'Schema mismatch', tone: 'danger' },
    recovering: { label: 'Recovering', tone: 'warning' },
  };

  const wakeTierLabels: Record<CollectorWakeTier, string> = {
    configuration_manager: 'Configuration Manager',
    registry: 'Registry',
    timer_only: 'Timer only',
  };

  const dockerStateLabels: Record<DockerCollectorState, string> = {
    probing: 'Docker probing',
    available: 'Docker available',
    reconnecting: 'Docker reconnecting',
    unavailable: 'Docker unavailable',
  };

  const numberFormatter = new Intl.NumberFormat('en-US');
  const count = (value: number): string => numberFormatter.format(value);
  const storageSize = (value: number): string => formatBytes(value).replace(' ', '\u00a0');
  const lastRun = (value: number | null): string => value === null ? 'Never' : formatAbsoluteTime(value);
  const warningTone = (value: number): string => value > 0 ? 'text-warning' : 'text-text-secondary';
  const dangerTone = (value: number): string => value > 0 ? 'text-danger' : 'text-text-secondary';

  onMount(() => {
    if (data !== undefined) return;
    let mounted = true;
    let detach = () => {};
    void diagnostics.attach()
      .then((next) => {
        if (mounted) detach = next;
        else next();
      })
      .catch(() => {
        // Store refreshes resolve errors into AsyncState. This only guards an
        // unexpected setup failure so mount never creates an unhandled promise.
      });
    return () => {
      mounted = false;
      detach();
    };
  });

  async function exportReport(): Promise<void> {
    exportState = null;
    try {
      await exportDiagnostics();
      exportState = 'Diagnostics report saved.';
    } catch {
      exportState = 'The diagnostics report could not be saved.';
    }
  }
</script>

<ScreenFrame title="Diagnostics" subtitle="Collector and local database health from this running app.">
  {#snippet actions()}
    <Button disabled={!live} aria-label={live ? 'Export Diagnostics' : 'Export Diagnostics in the live app'} onclick={() => void exportReport()}>
      Export diagnostics
    </Button>
  {/snippet}

  {#if live && diagnostics.refreshError}
    <div class="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-md bg-warning/15 px-4 py-3 text-sm text-text-primary" role="status" aria-live="polite">
      <p class="m-0 min-w-0 break-words"><strong>Live refresh failed.</strong> {diagnostics.refreshError} Values below are the last successful snapshot.</p>
      <Button size="sm" class="min-h-11" onclick={() => void diagnostics.refresh()}>Retry</Button>
    </div>
  {/if}

  <StateShell
    state={viewState}
    emptyTitle="Diagnostics are not available yet."
    emptyDescription="Collector health and local database state will appear after the app starts reporting."
    emptyIcon="diagnostics"
    errorTitle="Diagnostics could not be loaded"
    loadingRows={6}
  >
    {#snippet children(snap)}
      {@const dbStatus = databaseState[snap.db.state]}
      <section aria-labelledby="runtime-summary-title">
        <h2 id="runtime-summary-title" class="sr-only">Runtime summary</h2>
        <Card level={1} class="mb-4">
          <dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm @[720px]/shell:grid-cols-4">
            <div><dt class="text-caps text-text-muted">App version</dt><dd class="m-0 mt-1 text-text-primary font-tabular">{snap.app_version}</dd></div>
            <div><dt class="text-caps text-text-muted">WebView runtime</dt><dd class="m-0 mt-1 text-text-primary font-tabular">{snap.os_version}</dd></div>
            <div><dt class="text-caps text-text-muted">Database</dt><dd class="m-0 mt-1"><Badge tone={dbStatus.tone} dot>{dbStatus.label}</Badge></dd></div>
            <div><dt class="text-caps text-text-muted">Database size</dt><dd class="m-0 mt-1 text-text-primary font-tabular">{storageSize(snap.database_bytes)}</dd></div>
            <div><dt class="text-caps text-text-muted">Migration</dt><dd class="m-0 mt-1 text-text-primary font-tabular">{snap.schema_version}</dd></div>
            <div><dt class="text-caps text-text-muted">Rows rejected before queue</dt><dd class="m-0 mt-1 font-tabular {warningTone(snap.dropped_writes)}">{count(snap.dropped_writes)}</dd></div>
            <div><dt class="text-caps text-text-muted">Rows lost after commit</dt><dd class="m-0 mt-1 font-tabular {dangerTone(snap.failed_commit_rows)}">{count(snap.failed_commit_rows)}</dd></div>
            <div><dt class="text-caps text-text-muted">Atomic cycles rejected</dt><dd class="m-0 mt-1 font-tabular {warningTone(snap.cycle_admission_drops)}">{count(snap.cycle_admission_drops)}</dd></div>
            <div><dt class="text-caps text-text-muted">Atomic cycles failed</dt><dd class="m-0 mt-1 font-tabular {dangerTone(snap.failed_cycles)}">{count(snap.failed_cycles)}</dd></div>
            <div><dt class="text-caps text-text-muted">Write queue</dt><dd class="m-0 mt-1 text-text-primary font-tabular">{count(snap.write_queue_depth)}</dd></div>
            <div><dt class="text-caps text-text-muted">Working set</dt><dd class="m-0 mt-1 text-text-primary font-tabular">{storageSize(snap.current_working_set_bytes)}</dd></div>
            <div><dt class="text-caps text-text-muted">Peak working set</dt><dd class="m-0 mt-1 text-text-primary font-tabular">{storageSize(snap.peak_working_set_bytes)}</dd></div>
          </dl>
          {#if 'reason' in snap.db}
            <p class="mt-3 mb-0 break-words text-sm text-danger" role="alert">Database detail: {snap.db.reason}</p>
          {/if}
        </Card>
      </section>

      <section aria-labelledby="collector-health-title">
        <h2 id="collector-health-title" class="sr-only">Collector health</h2>
        {#if snap.collectors.length === 0}
          <Card level={1}>
            <p class="m-0 text-sm text-text-secondary" role="status">No collectors are registered in this runtime.</p>
          </Card>
        {:else}
          <Card level={1} padded={false} class="overflow-x-auto" role="region" tabindex={0} aria-label="Collector diagnostics table. Scroll horizontally for more columns.">
            <table class="min-w-full border-collapse text-sm">
              <caption class="sr-only">Live collector state, schedule, wake tier, Docker status, activity, row delivery and atomic cycle telemetry.</caption>
              <thead>
                <tr class="text-left text-caps text-text-muted">
                  <th rowspan="2" scope="col" class="px-3 py-2 font-medium">Collector</th>
                  <th rowspan="2" scope="col" class="px-3 py-2 font-medium">State</th>
                  <th rowspan="2" scope="col" class="px-3 py-2 font-medium">Cadence</th>
                  <th rowspan="2" scope="col" class="px-3 py-2 font-medium">Wake tier</th>
                  <th rowspan="2" scope="col" class="px-3 py-2 font-medium">Docker status</th>
                  <th rowspan="2" scope="col" class="px-3 py-2 font-medium">Last run</th>
                  <th rowspan="2" scope="col" class="px-3 py-2 font-medium">Duration</th>
                  <th rowspan="2" scope="col" class="px-3 py-2 text-right font-medium">Events accepted</th>
                  <th rowspan="2" scope="col" class="px-3 py-2 text-right font-medium">Consecutive failures</th>
                  <th colspan="2" scope="colgroup" class="border-l border-border-subtle px-3 py-2 text-center font-medium">Rows</th>
                  <th colspan="2" scope="colgroup" class="border-l border-border-subtle px-3 py-2 text-center font-medium">Atomic cycles</th>
                </tr>
                <tr class="border-t border-border-subtle text-left text-xs text-text-muted">
                  <th scope="col" class="border-l border-border-subtle px-3 py-2 text-right font-medium">Rejected before queue</th>
                  <th scope="col" class="px-3 py-2 text-right font-medium">Lost after commit</th>
                  <th scope="col" class="border-l border-border-subtle px-3 py-2 text-right font-medium">Rejected before admission</th>
                  <th scope="col" class="px-3 py-2 text-right font-medium">Failed after admission</th>
                </tr>
              </thead>
              <tbody>
                {#each snap.collectors as collector (collector.id)}
                  {@const status = collectorState[collector.state]}
                  <tr class="border-t border-border-subtle align-top">
                    <th scope="row" class="px-3 py-2 text-left font-medium text-text-primary">
                      {collector.descriptor.display_name}
                      {#if collector.last_error && (collector.state === 'failed' || collector.state === 'degraded')}
                        <span class="mt-1 block max-w-64 break-words text-xs leading-relaxed font-normal text-danger">{collector.last_error}</span>
                      {/if}
                    </th>
                    <td class="px-3 py-2"><Badge tone={status.tone} dot>{status.label}</Badge></td>
                    <td class="px-3 py-2 whitespace-nowrap text-text-secondary">{collectorCadenceLabel(collector.descriptor.cadence)}</td>
                    <td class="px-3 py-2 whitespace-nowrap text-text-secondary">{collector.details?.wake_tier ? wakeTierLabels[collector.details.wake_tier] : '—'}</td>
                    <td class="px-3 py-2 text-text-secondary">
                      {#if collector.id === 'docker' && collector.details?.docker_state}
                        <div class="whitespace-nowrap" role="group" aria-label="Docker status details">
                          <span class="block">{dockerStateLabels[collector.details.docker_state]}</span>
                          <span class="block text-xs text-text-muted">
                            {collector.details.docker_api_version ? `API ${collector.details.docker_api_version}` : 'API not negotiated'}
                          </span>
                        </div>
                      {:else}
                        <span aria-label="Not applicable">—</span>
                      {/if}
                    </td>
                    <td class="px-3 py-2 whitespace-nowrap text-text-secondary font-tabular">{lastRun(collector.last_run_at)}</td>
                    <td class="px-3 py-2 whitespace-nowrap text-text-secondary font-tabular">{collector.last_duration_ms === null ? 'Not run' : durationLabel(collector.last_duration_ms)}</td>
                    <td class="px-3 py-2 text-right text-text-secondary font-tabular">{count(collector.events_emitted)}</td>
                    <td class="px-3 py-2 text-right font-tabular {dangerTone(collector.consecutive_failures)}">{count(collector.consecutive_failures)}</td>
                    <td class="border-l border-border-subtle px-3 py-2 text-right font-tabular {warningTone(collector.queue_drops)}">{count(collector.queue_drops)}</td>
                    <td class="px-3 py-2 text-right font-tabular {dangerTone(collector.failed_commit_rows)}">{count(collector.failed_commit_rows)}</td>
                    <td class="border-l border-border-subtle px-3 py-2 text-right font-tabular {warningTone(collector.cycle_admission_drops)}">{count(collector.cycle_admission_drops)}</td>
                    <td class="px-3 py-2 text-right font-tabular {dangerTone(collector.failed_cycles)}">{count(collector.failed_cycles)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </Card>
        {/if}
      </section>
    {/snippet}
  </StateShell>
  {#if exportState}<p class="mt-3 text-sm text-text-secondary" role="status">{exportState}</p>{/if}
</ScreenFrame>
