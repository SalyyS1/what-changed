<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import { backendAvailable } from '../../lib/tauri/client';
  import { setRetention } from '../../lib/tauri/client';
  import { privacy } from '../../lib/stores/privacy.svelte';

  interface Props { active?: boolean }
  let { active = true }: Props = $props();
  let confirming = $state(false);
  let phrase = $state('');
  let retentionPending = $state(false);

  onMount(() => {
    if (active && backendAvailable()) void privacy.load();
  });

  async function removeEverything(): Promise<void> {
    if (phrase.trim().toLowerCase() !== 'delete') return;
    if (await privacy.removeEverything()) {
      confirming = false;
      phrase = '';
    }
  }

  async function chooseRetention(days: number): Promise<void> {
    const summary = privacy.summary;
    if (!active || summary === null || retentionPending || summary.retention.events_days === days) return;
    retentionPending = true;
    try {
      await setRetention({ ...summary.retention, events_days: days });
      await privacy.load();
    } catch (reason) {
      privacy.error = reason instanceof Error ? reason.message : 'Retention could not be saved.';
    } finally {
      retentionPending = false;
    }
  }
</script>

<div data-testid="privacy-screen">
  <Card level={1}>
    <p class="m-0 text-base text-text-primary">Your history stays on this PC.</p>
    <p class="mt-2 text-sm leading-relaxed text-text-secondary">No account, no background network calls, no telemetry, no crash reporting. The opt-in updater stays unavailable until its signed feed is configured. Sensitive values are replaced with [REDACTED] before comparison or storage.</p>
    <p class="mt-2 text-xs text-text-muted">For example: <code>DATABASE_URL</code> changed in <code>.env</code>, or <code>AWS_SECRET_ACCESS_KEY</code> changed; the value is never retained.</p>
  </Card>

  <StateShell state={privacy.state} emptyTitle="Privacy summary is not available yet." emptyDescription="The local database will provide counts after startup." loadingRows={2}>
    {#snippet children(summary)}
      <Card level={1} class="mt-3">
        <dl class="grid grid-cols-2 gap-3 text-sm @[680px]/shell:grid-cols-4">
          <div><dt class="text-caps text-text-muted">Events</dt><dd class="m-0 mt-1 text-text-primary">{summary.categories.reduce((total, row) => total + row.count, 0)}</dd></div>
          <div><dt class="text-caps text-text-muted">Checkpoints</dt><dd class="m-0 mt-1 text-text-primary">{summary.checkpoints}</dd></div>
          <div><dt class="text-caps text-text-muted">Investigations</dt><dd class="m-0 mt-1 text-text-primary">{summary.investigations}</dd></div>
          <div><dt class="text-caps text-text-muted">Database</dt><dd class="m-0 mt-1 text-text-primary">{summary.database_bytes} bytes</dd></div>
          <div><dt class="text-caps text-text-muted">App memory</dt><dd class="m-0 mt-1 text-text-primary">{summary.owner_workspace_bytes === null ? 'Size unavailable' : `${summary.owner_workspace_bytes} bytes`} · {summary.owner_store.status.replaceAll('_', ' ')}</dd></div>
        </dl>
        <p class="mt-3 selectable break-all text-xs text-text-muted">Data folder: {summary.data_path}</p>
        <div class="mt-4 border-t border-border-subtle pt-4">
          <p class="m-0 text-sm text-text-primary">Keep event history</p>
          <p class="m-0 mt-1 text-xs text-text-muted">Older routine events are removed automatically, and routine process start and stop events go after 3 days. If the database passes 1 GiB, the oldest of those routine events are removed first. Important history follows its longer retention policy.</p>
          <div class="mt-2 flex flex-wrap gap-2" role="group" aria-label="Event history retention">
            {#each [7, 30, 90] as days}
              <Button
                size="sm"
                variant={summary.retention.events_days === days ? 'primary' : 'ghost'}
                disabled={!active || retentionPending}
                aria-pressed={summary.retention.events_days === days}
                onclick={() => void chooseRetention(days)}
              >
                {days} days
              </Button>
            {/each}
          </div>
        </div>
        <div class="mt-4 flex flex-wrap gap-2">
          {#each summary.categories as row (row.category)}
            <Button size="sm" variant="ghost" disabled={privacy.pending || row.count === 0} onclick={() => void privacy.removeCategory(row.category)}>
              Delete {row.category} history ({row.count})
            </Button>
          {/each}
        </div>
      </Card>
    {/snippet}
  </StateShell>

  <Card level={1} class="mt-3">
    {#if !confirming}
      <Button variant="danger" disabled={!active || privacy.pending} onclick={() => confirming = true}>Delete everything now</Button>
    {:else}
      <p class="m-0 text-sm text-danger">This is irreversible. Type <strong>delete</strong> to remove event history, app workspaces, confirmed rules, requests and app-owned copies. Your project source and files exported elsewhere are kept.</p>
      <div class="mt-3 flex flex-wrap gap-2">
        <input class="min-h-11 rounded-md border border-border-subtle bg-surface-1 px-3 text-sm" aria-label="Type delete to confirm" bind:value={phrase} autocomplete="off" />
        <Button variant="danger" disabled={privacy.pending || phrase.trim().toLowerCase() !== 'delete'} onclick={() => void removeEverything()}>Confirm deletion</Button>
        <Button variant="ghost" disabled={privacy.pending} onclick={() => { confirming = false; phrase = ''; }}>Cancel</Button>
      </div>
    {/if}
    {#if privacy.error}<p class="mt-3 text-sm text-danger" role="alert">{privacy.error}</p>{/if}
  </Card>
</div>
