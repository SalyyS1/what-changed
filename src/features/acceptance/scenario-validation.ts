import type { OriginDto, OwnerCandidateRevisionDto, OwnerEnvironmentRevisionDto, ReadBackAdapterDto, RecipeStepDto, ScenarioRecipeDto } from '../../lib/tauri/bindings';
import { originKey, uniqueOrigins } from '../owner-workspace/owner-origins';

export const MAX_STEP_ASSERTIONS = 100;
export const MAX_SCENARIO_ASSERTIONS = 1000;
export const assertionCount = (recipe: ScenarioRecipeDto): number => recipe.steps.reduce((count, step) => count + step.assertions.length, 0);
export const canAddAssertion = (step: RecipeStepDto, total: number): boolean => step.assertions.length < MAX_STEP_ASSERTIONS && total < MAX_SCENARIO_ASSERTIONS;
export function actionAdapters(action: RecipeStepDto['action']['kind'], adapters: ReadBackAdapterDto[]): ReadBackAdapterDto[] {
  return adapters.filter((adapter) => adapter.effect === (action === 'read_back' ? 'read_only' : action === 'deny_other_role' ? 'read_denied' : null));
}

/** A scenario draft uses the selected candidate's exact current environment only. */
export function candidateOrigins(environments: OwnerEnvironmentRevisionDto[], candidate?: OwnerCandidateRevisionDto | null): { origins: OriginDto[]; message: string | null } {
  if (!candidate) return { origins: [], message: 'Select an app candidate before adding read-back or access routes.' };
  const revisions = environments.filter((item) => item.id === candidate.environment_id);
  const environment = revisions.find((item) => item.revision === candidate.environment_revision);
  if (!environment) return { origins: [], message: 'The selected candidate’s exact preview revision is unavailable. Save preview settings and select a matching candidate.' };
  if (revisions.some((item) => item.revision > environment.revision)) return { origins: [], message: 'The selected candidate uses an older preview revision. Select a candidate for the current preview settings before defining routes.' };
  if (environment.revoked) return { origins: [], message: 'The selected candidate’s preview settings are revoked. Select a candidate for an active preview before defining routes.' };
  return { origins: uniqueOrigins(environment.origins), message: null };
}

/** Explain editing compatibility before review; the backend remains the validation authority. */
export function recipeCompatibilityIssues(recipe: ScenarioRecipeDto, origins: OriginDto[]): string[] {
  const issues: string[] = [];
  const report = (message: string): void => { if (issues.length < 20) issues.push(message); };
  if (recipe.steps.length > 50) report('Use at most 50 steps in one scenario.');
  if (assertionCount(recipe) > MAX_SCENARIO_ASSERTIONS) report('Use at most 1000 expected checks across this scenario.');
  if (recipe.role_refs.length > 10) report('Use at most 10 declared test roles.');
  if (recipe.fixture_refs.length > 50) report('Use at most 50 test-data references.');
  if (recipe.adapters.length > 16) report('Use at most 16 reviewed routes.');
  const approved = new Set(origins.map(originKey));
  for (const [index, adapter] of recipe.adapters.entries()) {
    if (!approved.has(originKey(adapter.origin))) report(`Route ${index + 1}: choose an exact origin from the selected candidate’s current preview settings.`);
    if (!adapter.role_ref.trim() || !recipe.role_refs.includes(adapter.role_ref)) report(`Route ${index + 1}: its test role must exactly match a declared test-role reference.`);
    if (!adapter.allowed_fields.length || adapter.allowed_fields.length > 50 || adapter.allowed_fields.some((field) => !field.trim())) report(`Route ${index + 1}: select between 1 and 50 nonblank response fields.`);
  }
  for (const [index, step] of recipe.steps.entries()) {
    if (step.assertions.length > MAX_STEP_ASSERTIONS) report(`Step ${index + 1}: use at most 100 expected checks.`);
    const action = step.action;
    const adapter = action.kind === 'read_back' || action.kind === 'deny_other_role' ? recipe.adapters.find((item) => item.id === action.adapter_id) : undefined;
    if ((action.kind === 'read_back' || action.kind === 'deny_other_role') && !adapter) report(`Step ${index + 1}: select an existing reviewed route for this action.`);
    if (adapter && !actionAdapters(action.kind, [adapter]).length) report(`Step ${index + 1}: select a route whose purpose matches ${action.kind === 'read_back' ? 'reading stored fields' : 'checking denied access'}.`);
    for (const [position, named] of step.assertions.entries()) {
      const assertion = named.assertion; const label = `Step ${index + 1}, check ${position + 1}`;
      if (assertion.kind === 'field_equals') {
        if (action.kind !== 'read_back' || adapter?.effect !== 'read_only') report(`${label}: a stored-field comparison needs a read-back action and read-only route.`);
        else if (!adapter.allowed_fields.includes(assertion.field)) report(`${label}: the stored field must be selected in that route’s allowed response fields.`);
      }
      if (assertion.kind === 'denied') {
        if (action.kind !== 'deny_other_role' || adapter?.effect !== 'read_denied') report(`${label}: a denied-access check needs a deny-other-role action and denied-access route.`);
        else if (assertion.absent_fields.some((field) => !adapter.allowed_fields.includes(field))) report(`${label}: every absent field must be selected in that route’s allowed response fields.`);
      }
    }
  }
  return issues;
}
