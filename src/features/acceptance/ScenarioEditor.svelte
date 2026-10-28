<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { ConfirmOwnerScenarioInput, OriginDto, OwnerCapabilityDto, OwnerRuleRevisionDto, RuleRefDto, ScenarioRecipeDto, ScenarioRevisionDto } from '../../lib/tauri/bindings';
  import { latestOwnerRules } from '../../lib/stores/owner-workspace.svelte';
  import RecipeStepEditor from './RecipeStepEditor.svelte';
  import AdapterEditor from './AdapterEditor.svelte';
  import RecipeSummary from './RecipeSummary.svelte';
  import { newRecipeStep, scenarioTemplate, splitReferences, type ScenarioTemplate } from './scenario-recipes';
  import { assertionCount, recipeCompatibilityIssues } from './scenario-validation';

  export type ScenarioFields = Omit<ConfirmOwnerScenarioInput, 'operation_id' | 'id' | 'app_id' | 'expected_version'>;
  interface Props { capabilities: OwnerCapabilityDto[]; rules: OwnerRuleRevisionDto[]; scenarios: ScenarioRevisionDto[]; origins: OriginDto[]; saving?: boolean; disabled?: boolean; error?: string | null; onConfirm: (fields: ScenarioFields, previous?: ScenarioRevisionDto) => Promise<boolean>; }
  let { capabilities, rules, scenarios, origins, saving = false, disabled = false, error = null, onConfirm }: Props = $props();
  let editing = $state(false);
  let previous = $state<ScenarioRevisionDto | undefined>();
  let title = $state(''); let expected = $state(''); let capabilityId = $state('');
  let confirmedBy = $state(''); let selectedRules = $state<RuleRefDto[]>([]);
  let recipe = $state<ScenarioRecipeDto>({ schema_version: 1, role_refs: [], fixture_refs: [], steps: [], adapters: [] });
  let confirmation = $state(false); let acknowledged = $state(false); let message = $state<string | null>(null);
  const latestRules = $derived(latestOwnerRules(rules));
  const latestScenarios = $derived(scenarios.filter((item) => !scenarios.some((other) => other.id === item.id && other.revision > item.revision)));
  const capabilityOptions = $derived(capabilities.map((item) => ({ value: item.id, label: item.label })));
  const totalAssertions = $derived(assertionCount(recipe));
  const compatibilityIssues = $derived(recipeCompatibilityIssues(recipe, origins));

  function start(template?: ScenarioTemplate): void {
    const proposal = template ? scenarioTemplate(template, origins[0]) : null;
    if (template && !proposal) { message = 'Save an authorized preview origin before using a read-back or access template.'; return; }
    previous = undefined; title = proposal?.title ?? ''; expected = proposal?.expected ?? '';
    capabilityId = capabilities[0]?.id ?? ''; confirmedBy = ''; selectedRules = latestRules.map(({ rule_id, revision }) => ({ rule_id, revision }));
    recipe = proposal?.recipe ?? { schema_version: 1, role_refs: [], fixture_refs: [], steps: [newRecipeStep()], adapters: [] };
    editing = true; message = null;
  }
  function revise(scenario: ScenarioRevisionDto): void {
    previous = scenario; title = scenario.title; expected = scenario.expected_behavior; capabilityId = scenario.capability_id;
    confirmedBy = ''; selectedRules = scenario.rules.map((ref) => ({ ...ref })); recipe = structuredClone($state.snapshot(scenario.recipe)); editing = true; message = null;
  }
  function toggleRule(rule: OwnerRuleRevisionDto, selected: boolean): void {
    selectedRules = selectedRules.filter((ref) => ref.rule_id !== rule.rule_id);
    if (selected) selectedRules.push({ rule_id: rule.rule_id, revision: rule.revision });
  }
  function review(): void {
    if (!title.trim() || !expected.trim() || !capabilityId || !confirmedBy.trim()) { message = 'Add a title, expected behavior, capability and the name of the confirming owner.'; return; }
    if (!recipe.steps.length || !recipe.steps.some((step) => step.assertions.length)) { message = 'Add at least one step and one named expected check.'; return; }
    if (compatibilityIssues.length) { message = 'Correct the route and expected-check issues below before reviewing this revision.'; return; }
    acknowledged = false; confirmation = true; message = null;
  }
  async function confirm(): Promise<void> {
    if (!acknowledged || compatibilityIssues.length) return;
    const saved = await onConfirm({ capability_id: capabilityId, title: title.trim(), expected_behavior: expected.trim(), confirmed_by: confirmedBy.trim(), method: 'owner_manual', recipe: $state.snapshot(recipe), rules: $state.snapshot(selectedRules) }, previous);
    if (saved) { confirmation = false; editing = false; message = 'Scenario revision confirmed. No observation has been recorded for this new revision.'; }
  }
</script>

<Card>
  <h2 class="font-medium text-text-primary">Acceptance scenarios</h2><p class="mt-1 text-sm text-text-secondary">Agree on expected behavior before checking it. Confirming a scenario does not run it. You can observe it manually or explicitly authorize a supported test browser later.</p>
  {#if latestScenarios.length}<ul class="mt-4 space-y-3">{#each latestScenarios as scenario (scenario.id)}<li class="rounded-md border border-border-subtle p-3"><div class="flex flex-wrap gap-2"><h3 class="text-sm font-medium text-text-primary">{scenario.title}</h3><Badge tone="neutral">Confirmed revision {scenario.revision}</Badge><Badge tone="neutral">{scenario.method.replaceAll('_', ' ')}</Badge></div><p class="mt-2 whitespace-pre-wrap text-sm text-text-secondary">{scenario.expected_behavior}</p><p class="mt-1 text-xs text-text-muted">{scenario.recipe.steps.length} steps · {scenario.recipe.steps.reduce((count, step) => count + step.assertions.length, 0)} named checks · confirmed by {scenario.confirmed_by}</p><Button class="mt-2" size="sm" variant="ghost" disabled={disabled || saving} onclick={() => revise(scenario)}>Propose a revision</Button></li>{/each}</ul>{:else}<p class="mt-3 text-sm text-text-muted">No confirmed scenarios. Start from a template or describe a bounded check yourself.</p>{/if}
  <div class="mt-4 flex flex-wrap gap-2"><Button size="sm" variant="secondary" disabled={disabled || saving || !capabilities.length} onclick={() => start('booking-create')}>Booking creation template</Button><Button size="sm" variant="secondary" disabled={disabled || saving || !capabilities.length} onclick={() => start('booking-read-back')}>Stored booking template</Button><Button size="sm" variant="secondary" disabled={disabled || saving || !capabilities.length} onclick={() => start('booking-access')}>Access isolation template</Button><Button size="sm" variant="ghost" disabled={disabled || saving || !capabilities.length} onclick={() => start()}>New scenario</Button></div>
  {#if !capabilities.length}<p class="mt-2 text-xs text-warning">Add a capability before confirming a scenario for it.</p>{/if}
  {#if message}<p class="mt-3 text-sm text-text-secondary" role="status">{message}</p>{/if}
  {#if editing}
    <fieldset class="mt-4 space-y-4 border-t border-border-subtle pt-4" disabled={disabled || saving}><legend class="sr-only">Draft scenario</legend>
      <div class="flex flex-wrap gap-2"><Badge tone="neutral">Unconfirmed draft</Badge><Badge tone="neutral">{previous ? `Proposed revision ${previous.revision + 1}` : 'First revision'}</Badge><Badge tone="neutral">Owner-reviewed template or wording</Badge></div>
      <label class="block text-sm text-text-secondary">Scenario title<Input bind:value={title} maxlength={200} /></label><label class="block text-sm text-text-secondary">Expected business behavior<Textarea bind:value={expected} rows={3} maxlength={4000} /></label><Select bind:value={capabilityId} options={capabilityOptions} label="Scenario capability" disabled={disabled || saving || previous !== undefined} />
      <label class="block text-sm text-text-secondary">Test-role references (comma separated, no credentials)<Input value={recipe.role_refs.join(', ')} oninput={(event) => (recipe.role_refs = splitReferences(event.currentTarget.value))} /></label><label class="block text-sm text-text-secondary">Isolated test-data references (comma separated)<Input value={recipe.fixture_refs.join(', ')} oninput={(event) => (recipe.fixture_refs = splitReferences(event.currentTarget.value))} /></label>
      <fieldset class="space-y-2"><legend class="mb-2 text-sm font-medium text-text-primary">Confirmed rules this scenario checks</legend>{#each latestRules as rule (rule.rule_id)}<label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" checked={selectedRules.some((ref) => ref.rule_id === rule.rule_id)} onchange={(event) => toggleRule(rule, event.currentTarget.checked)} /><span>{rule.expected_behavior} · revision {rule.revision}{#if selectedRules.some((ref) => ref.rule_id === rule.rule_id && ref.revision !== rule.revision)} · draft still references an older revision; deselect and select to use this wording{/if}</span></label>{/each}</fieldset>
      <AdapterEditor bind:adapters={recipe.adapters} {origins} disabled={disabled || saving} />
      <div class="space-y-3">{#each recipe.steps as step, index (step.id)}<RecipeStepEditor bind:step={recipe.steps[index]!} {index} {totalAssertions} adapters={recipe.adapters} disabled={disabled || saving} onRemove={() => (recipe.steps = recipe.steps.filter((item) => item.id !== step.id))} />{/each}<Button size="sm" variant="secondary" disabled={disabled || saving || recipe.steps.length >= 50} onclick={() => (recipe.steps = [...recipe.steps, newRecipeStep()])}>Add step</Button><p class="text-xs text-text-muted">{totalAssertions} of 1000 expected checks · at most 100 per step.</p></div>
      <label class="block text-sm text-text-secondary">Confirmed by<Input bind:value={confirmedBy} placeholder="Your name or owner label" /></label>
      {#if error}<p class="text-sm text-danger" role="alert">{error} The draft is still here.</p>{/if}
      {#if compatibilityIssues.length}<div class="space-y-2 text-sm text-warning" role="status"><p class="font-medium">Correct these draft issues before review:</p><ul class="list-inside list-disc">{#each compatibilityIssues as issue, index (index)}<li>{issue}</li>{/each}</ul><p class="text-xs">Changing a route or action keeps your expected checks. Correct each mismatch explicitly.</p></div>{/if}
      <div class="flex flex-wrap gap-2"><Button variant="primary" disabled={disabled || saving} onclick={review}>Review revision</Button><Button variant="ghost" onclick={() => (editing = false)}>Close draft</Button></div>
    </fieldset>
  {/if}
</Card>

<Dialog bind:open={confirmation} title="Confirm this scenario revision?" description="Previous observations stay attached to their original expected checks. Nothing becomes passed by confirming a revision." width="lg">
  <div class="space-y-4">{#if previous}<div class="rounded-md border border-border-subtle p-3"><h3 class="text-sm font-medium text-text-primary">Current confirmed revision {previous.revision}</h3><p class="mt-1 whitespace-pre-wrap text-sm text-text-secondary">{previous.expected_behavior}</p><RecipeSummary recipe={previous.recipe} /><p class="mt-3 text-xs text-text-muted">Confirmed by {previous.confirmed_by} · method: {previous.method.replaceAll('_', ' ')} · rule revisions: {previous.rules.map((ref) => ref.revision).join(', ') || 'none'}</p></div>{/if}<div class="rounded-md border border-border-active p-3"><h3 class="text-sm font-medium text-text-primary">Proposed revision {previous ? previous.revision + 1 : 1}: {title}</h3><p class="mt-1 whitespace-pre-wrap text-sm text-text-primary">{expected}</p><RecipeSummary {recipe} /><p class="mt-3 text-xs text-text-muted">Confirming owner: {confirmedBy} · manual observation · rule revisions: {selectedRules.map((ref) => ref.revision).join(', ') || 'none'}</p></div><label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" bind:checked={acknowledged} />I reviewed the old and proposed behavior, roles, routes, test data and checks. This is the scenario I want to confirm.</label>{#if error}<p class="text-sm text-danger" role="alert">{error}</p>{/if}</div>
  {#snippet footer()}<Button variant="ghost" onclick={() => (confirmation = false)}>Keep editing</Button><Button variant="primary" disabled={!acknowledged || saving || compatibilityIssues.length > 0} onclick={() => void confirm()}>{saving ? 'Confirming…' : 'Confirm revision'}</Button>{/snippet}
</Dialog>
