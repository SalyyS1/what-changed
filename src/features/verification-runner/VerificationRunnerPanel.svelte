<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Select from '../../components/ui/Select.svelte';
  import type { OwnerChangeReviewDto, PreparePreviewRunInput, ScenarioRevisionDto, RunnerRouteDto } from '../../lib/tauri/bindings';
  import { PreviewRunnerStore } from '../../lib/stores/preview-runner.svelte';
  import { runnerFixtureFields, runnerOrigin, suggestedRunnerRoutes } from './runner-scope';
  import RunnerScopeEditor from './RunnerScopeEditor.svelte';
  import RunConsent from './RunConsent.svelte';
  import RunnerStatus from './RunnerStatus.svelte';
  interface Props { change: OwnerChangeReviewDto; scenarios: ScenarioRevisionDto[]; disabled?: boolean; model?: PreviewRunnerStore; onRunChanged?: (runId: string) => void | Promise<void>; }
  let { change, scenarios, disabled = false, model = new PreviewRunnerStore(), onRunChanged }: Props = $props();
  let routes = $state<RunnerRouteDto[]>([]); let fixture = $state<Record<string, string>>({});
  let mode = $state('run'); let consentOpen = $state(false); let captureScreenshots = $state(false);
  let notifiedTerminalRun = $state<string | null>(null);
  const origins = $derived(change.environment?.origins.map(runnerOrigin) ?? []);
  const eligible = $derived(Boolean(!disabled && change.change.candidate_id && scenarios.length && change.environment && !change.environment.revoked));
  const scopeKey = $derived(JSON.stringify([change.app.id, change.change.id, change.change.version, change.change.candidate_id, change.environment?.revision, scenarios.map((item) => [item.id, item.revision])]));
  $effect(() => {
    void scopeKey;
    untrack(() => {
      model.dispose(); consentOpen = false;
      routes = suggestedRunnerRoutes(change, scenarios);
      fixture = Object.fromEntries(runnerFixtureFields(scenarios).map((field) => [field, '']));
    });
  });
  $effect(() => {
    const session = model.session;
    if (!session || !model.running) return;
    let live = true;
    let timer: ReturnType<typeof setTimeout>;
    const tick = async (): Promise<void> => { await model.refresh(); if (live) timer = setTimeout(() => void tick(), 700); };
    timer = setTimeout(() => void tick(), 700);
    return () => { live = false; clearTimeout(timer); };
  });
  $effect(() => {
    const session = model.session;
    if (session && !model.running && session.run_id !== notifiedTerminalRun) {
      notifiedTerminalRun = session.run_id;
      untrack(() => void onRunChanged?.(session.run_id));
    }
  });
  onDestroy(() => model.dispose());

  function input(): PreparePreviewRunInput | null {
    const saved = change.change;
    if (!eligible || !saved.candidate_id) return null;
    return { run: { operation_id: crypto.randomUUID(), id: crypto.randomUUID(), app_id: saved.app_id, change_id: saved.id,
      expected_change_version: saved.version, candidate_id: saved.candidate_id,
      scenarios: scenarios.map(({ id, revision }) => ({ scenario_id: id, revision })), retry_of: null },
      routes: $state.snapshot(routes), fixture: $state.snapshot(fixture), mode, capture_screenshots: captureScreenshots };
  }
  async function prepare(): Promise<void> { const request = input(); if (request && await model.prepare(request)) consentOpen = true; }
  async function grant(): Promise<void> { const session = await model.grant(); consentOpen = false; if (session) await onRunChanged?.(session.run_id); }
</script>

<Card class="mt-4">
  <h3 class="font-medium text-text-primary">Check in an isolated test browser</h3>
  <p class="mt-1 text-sm text-text-secondary">Run the selected confirmed scenarios, or select locators for a proposed revision. Nothing starts until you review the exact scope and grant permission.</p>
  <p class="mt-2 text-xs text-warning">This candidate supports literal loopback test URLs such as http://127.0.0.1:3000/. Remote previews remain unavailable pending network confinement verification. Packaged runtime availability is checked before execution.</p>
  {#if !eligible}<p class="mt-3 text-sm text-text-secondary">Bind a candidate for this saved request and select at least one confirmed scenario.</p>{/if}
  <div class="mt-3"><Select bind:value={mode} label="Test browser purpose" options={[{ value: 'run', label: 'Run confirmed checks' }, { value: 'record', label: 'Select locators without recording values' }]} disabled={!eligible || model.busy || model.running || model.plan !== null} /></div>
  <RunnerScopeEditor bind:routes bind:fixture {origins} disabled={!eligible || model.busy || model.running || model.plan !== null} />
  <label class="mt-4 flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" bind:checked={captureScreenshots} disabled={!eligible || model.busy || model.running || model.plan !== null} />Allow me to request screenshots during this session. Each capture needs a separate review before retention; it may contain sensitive test data.</label>
  <div class="mt-4 flex flex-wrap gap-2"><Button variant="primary" disabled={!eligible || model.busy || model.running || Object.values(fixture).some((value) => !value.trim())} onclick={() => void prepare()}>{model.busy ? 'Preparing…' : 'Review test session'}</Button>{#if model.plan}<Button variant="secondary" disabled={model.busy} onclick={() => (consentOpen = true)}>Open permission review</Button><Button variant="ghost" disabled={model.busy} onclick={() => model.revise()}>Edit scope</Button><Button variant="secondary" disabled={model.busy} onclick={() => void model.previewExport()}>Review runnable export</Button>{/if}</div>
  {#if model.error}<p class="mt-3 text-sm text-danger" role="alert">{model.error}</p>{/if}
  {#if model.recipe}<section class="mt-4 space-y-3" aria-label="Runnable export review"><h4 class="text-sm font-medium text-text-primary">Runnable test files</h4><p class="text-sm text-text-secondary">Review every file before saving. Exporting does not run tests or grant permission on another machine. Credentials and live session authority are excluded.</p>{#each model.recipe.limitations as limitation (limitation)}<p class="text-xs text-warning">{limitation}</p>{/each}{#each model.recipe.files as file (file.path)}<details><summary class="cursor-pointer text-sm text-text-primary">{file.path}</summary><pre class="selectable mt-2 max-h-64 overflow-auto rounded-md bg-surface-2 p-3 text-xs text-text-secondary">{file.content}</pre></details>{/each}<Button variant="secondary" disabled={model.busy} onclick={() => void model.saveExport(model.recipe!.digest)}>Save reviewed test ZIP…</Button>{#if model.saved}<p class="text-sm text-text-secondary" role="status">Saved to {model.saved}</p>{/if}</section>{/if}
  <RunnerStatus {model} />
</Card>
{#if model.plan}{#key model.plan.consent_id}<RunConsent bind:open={consentOpen} plan={model.plan} busy={model.busy} onGrant={() => void grant()} />{/key}{/if}
