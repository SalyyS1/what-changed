import type { OwnerChangeReviewDto, OwnerEvidenceReviewDto, OwnerManualRunDto, ScenarioRevisionDto } from '../../lib/tauri/bindings';
import { fixtureApp, fixtureDraft, fixtureRule } from '../owner-workspace/owner-workspace-fixtures';

/** Synthetic evidence for focused tests. Production stores never import this module. */
export const fixtureScenario = (revision = 1): ScenarioRevisionDto => ({
  id: 'scenario-a', app_id: 'app-a', capability_id: 'capability-a', revision, title: 'Create a booking',
  expected_behavior: 'The named booking confirmation appears.', confirmed_by: 'Synthetic owner', method: 'owner_manual',
  recipe: { schema_version: 1, role_refs: ['customer-test'], fixture_refs: ['booking-test'], adapters: [],
    steps: [{ id: 'step-a', action: { kind: 'navigate', path: '/bookings' }, assertions: [{ id: 'assertion-a', expected_behavior: 'The booking confirmation is visible.', assertion: { kind: 'visible', locator: { kind: 'test_id', value: 'booking-confirmation' } } }] }] },
  rules: [{ rule_id: 'rule-a', revision: 1 }], confirmed_at: 1,
});
export const fixtureCandidate = () => ({ id: 'candidate-a', app_id: 'app-a', environment_id: 'environment-a', environment_revision: 1, identity: 'Synthetic owner label', source: 'owner_label' as const, trust: 'self_reported' as const, observed_at: 1 });
export const fixtureEnvironment = () => ({ id: 'environment-a', app_id: 'app-a', revision: 1, label: 'Synthetic test preview', preview_url: 'http://localhost:3000/', origins: [{ scheme: 'http', host: 'localhost', port: 3000 }], kind: 'local' as const, revoked: false, created_at: 1 });
export function fixtureChangeReview(appId = 'app-a'): OwnerChangeReviewDto {
  return { app: { ...fixtureApp(appId), current_candidate_id: 'candidate-a' }, change: { ...fixtureDraft(appId), candidate_id: 'candidate-a' },
    preserved_rules: [fixtureRule()], candidate: fixtureCandidate(), environment: fixtureEnvironment() };
}
export function fixtureManualRun(appId = 'app-a'): OwnerManualRunDto {
  return { id: 'run-a', app_id: appId, change_id: 'change-a', version: 1, method: 'owner_manual', lifecycle: 'running', retry_of: null,
    manifest: { schema_version: 1, change: fixtureChangeReview(appId).change, candidate: fixtureCandidate(), environment: fixtureEnvironment(),
      rules: [fixtureRule()], scenarios: [fixtureScenario()], fixture_schema_digest: 'synthetic-fixture-digest', semantics_version: 'manual-v1' },
    manifest_digest: 'synthetic-manifest-digest', started_at: 1, ended_at: null, execution: null };
}
export function fixtureEvidenceReview(appId = 'app-a'): OwnerEvidenceReviewDto {
  return { run: fixtureManualRun(appId), freshness: 'unknown', freshness_reasons: ['Manual observations do not establish the deployed build.'], status: 'not_run',
    counts: { passed: 0, failed: 0, blocked: 0, not_run: 1 }, scenarios: [{ scenario: fixtureScenario(), status: 'not_run', result: null }], decisions: [],
    unresolved_scenarios: [{ scenario_id: 'scenario-a', revision: 1 }], can_accept: false, can_accept_with_exceptions: false,
    decision_blockers: ['Finish the observation attempt before accepting exceptions.'], limitations: ['No automated runner was started.'] };
}
