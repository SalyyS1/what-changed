<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Separator from '../../components/ui/Separator.svelte';
  import Switch from '../../components/ui/Switch.svelte';
  import { formatRelativeTime } from '../../lib/formatting/relative';
  import type { CollectorStatusDto } from '../../lib/tauri/bindings';
  import {
    collectorCadenceLabel,
    collectorCostPresentation,
    collectorStatePresentation,
    durationLabel,
  } from './collector-presentation';

  interface Props {
    rows: readonly CollectorStatusDto[];
    disabled?: boolean;
    pending?: ReadonlySet<string>;
    errors?: Readonly<Record<string, string>>;
    ontoggle: (id: string, enabled: boolean) => void;
  }

  let { rows, disabled = false, pending = new Set(), errors = {}, ontoggle }: Props = $props();

  function blockedReason(row: CollectorStatusDto): string | null {
    if (row.requires_admin || row.descriptor.requires_admin) return 'Requires administrator access';
    if (row.state === 'unavailable' && !row.enabled) return 'Not available on this PC';
    return null;
  }

  function watchedCopy(row: CollectorStatusDto): string {
    if (row.id === 'docker' && row.details?.docker_state === 'unavailable') {
      return "Docker isn't running. We'll start recording container changes when it does.";
    }
    return row.descriptor.what_it_watches;
  }
</script>

<Card level={1} padded={false}>
  {#each rows as row, index (row.id)}
    {@const state = collectorStatePresentation(row.state)}
    {@const cost = collectorCostPresentation(row.descriptor.cost_hint)}
    {@const blockedReasonText = blockedReason(row)}
    {@const blocked = blockedReasonText !== null}
    {#if index > 0}<Separator />{/if}
    <article class="px-4 py-4" aria-labelledby={`collector-${row.id}-name`}>
      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <h3 id={`collector-${row.id}-name`} class="text-base font-medium text-text-primary">{row.descriptor.display_name}</h3>
            <Badge tone={state.tone} dot>{state.label}</Badge>
            <Badge tone={cost.tone}>{cost.label}</Badge>
          </div>
          <p class="mt-1 text-sm text-text-secondary">{watchedCopy(row)}</p>
          <p class="mt-1 text-xs leading-relaxed text-text-muted">{row.descriptor.why_it_matters}</p>
        </div>
        <div class="flex shrink-0 flex-col items-end gap-1">
          <Switch
            checked={row.enabled}
            label={`${row.enabled ? 'Stop watching' : 'Watch'} ${row.descriptor.display_name}`}
            disabled={disabled || pending.has(row.id) || blocked}
            onchange={(enabled) => ontoggle(row.id, enabled)}
          />
          {#if pending.has(row.id)}
            <span class="text-xs text-text-muted" role="status">Updating…</span>
          {:else if blocked}
            <span class="max-w-36 text-right text-xs text-text-muted">{blockedReasonText}</span>
          {/if}
        </div>
      </div>

      <dl class="mt-3 grid grid-cols-2 gap-x-4 gap-y-2 rounded-md bg-surface-2 px-3 py-2 text-xs @[680px]/shell:grid-cols-4">
        <div><dt class="text-text-muted">Schedule</dt><dd class="m-0 mt-0.5 text-text-secondary">{collectorCadenceLabel(row.descriptor.cadence)}</dd></div>
        <div><dt class="text-text-muted">Last run</dt><dd class="m-0 mt-0.5 text-text-secondary font-tabular">{row.last_run_at === null ? 'Never' : formatRelativeTime(row.last_run_at)}</dd></div>
        <div><dt class="text-text-muted">Duration</dt><dd class="m-0 mt-0.5 text-text-secondary font-tabular">{durationLabel(row.last_duration_ms)}</dd></div>
        <div><dt class="text-text-muted">Events accepted</dt><dd class="m-0 mt-0.5 text-text-secondary font-tabular">{row.events_emitted.toLocaleString('en-US')}</dd></div>
      </dl>

      {#if row.consecutive_failures > 0 || row.queue_drops > 0 || row.failed_commit_rows > 0 || row.cycle_admission_drops > 0 || row.failed_cycles > 0}
        <div class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs" aria-label="Collector problems">
          {#if row.consecutive_failures > 0}<span class="text-danger">{row.consecutive_failures} consecutive {row.consecutive_failures === 1 ? 'failure' : 'failures'}</span>{/if}
          {#if row.queue_drops > 0}<span class="text-warning font-tabular">{row.queue_drops} queue {row.queue_drops === 1 ? 'drop' : 'drops'}</span>{/if}
          {#if row.failed_commit_rows > 0}<span class="text-danger font-tabular">{row.failed_commit_rows} {row.failed_commit_rows === 1 ? 'row' : 'rows'} lost after a failed database commit</span>{/if}
          {#if row.cycle_admission_drops > 0}<span class="text-warning font-tabular">{row.cycle_admission_drops} atomic {row.cycle_admission_drops === 1 ? 'cycle' : 'cycles'} rejected before database admission</span>{/if}
          {#if row.failed_cycles > 0}<span class="text-danger font-tabular">{row.failed_cycles} admitted atomic {row.failed_cycles === 1 ? 'cycle' : 'cycles'} failed</span>{/if}
        </div>
      {/if}
      {#if row.last_error && (row.state === 'failed' || row.state === 'degraded')}
        <p class="mt-2 text-xs text-danger">Last error: {row.last_error}</p>
      {/if}
      {#if errors[row.id]}
        <p class="mt-2 text-xs text-danger" role="alert">{errors[row.id]}</p>
      {/if}
    </article>
  {/each}
</Card>
