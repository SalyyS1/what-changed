<script lang="ts">
  import type { ScenarioRecipeDto } from '../../lib/tauri/bindings';
  import { describeAction, describeAssertion } from './scenario-recipes';
  interface Props { recipe: ScenarioRecipeDto; }
  let { recipe }: Props = $props();
</script>

<div class="mt-3 space-y-3 text-xs text-text-secondary">
  <p>Test roles: {recipe.role_refs.join(', ') || 'None declared.'}</p>
  <p>Test-data references: {recipe.fixture_refs.join(', ') || 'None declared.'}</p>
  <ol class="list-inside list-decimal space-y-3">{#each recipe.steps as step (step.id)}<li><span>{describeAction(step.action, recipe.adapters)}</span><ul class="mt-1 list-inside list-disc space-y-1">{#each step.assertions as assertion (assertion.id)}<li>{assertion.expected_behavior} · exact check: {describeAssertion(assertion.assertion)}</li>{:else}<li>No expected check at this step.</li>{/each}</ul></li>{/each}</ol>
  {#if recipe.adapters.length}<div><h4 class="font-medium text-text-primary">Declared routes</h4><ul class="mt-1 space-y-2">{#each recipe.adapters as adapter (adapter.id)}<li>{adapter.origin.scheme}://{adapter.origin.host}:{adapter.origin.port}{adapter.path_template} · role {adapter.role_ref} · {adapter.effect.replaceAll('_', ' ')} · allowed fields: {adapter.allowed_fields.join(', ')}</li>{/each}</ul></div>{/if}
</div>
