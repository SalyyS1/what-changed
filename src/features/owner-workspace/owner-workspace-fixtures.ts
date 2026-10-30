import type { OwnerAppDto, OwnerRuleRevisionDto, OwnerChangeDraftDto } from '../../lib/tauri/bindings';

/** Isolated synthetic test context. Never used by the production singleton. */
export const fixtureApp = (id = 'app-a'): OwnerAppDto => ({
  id, name: `Synthetic ${id}`, description: 'Synthetic booking test context', legacy_project_id: null,
  current_candidate_id: null, version: 1, archived_at: null, created_at: 1, updated_at: 1,
});
export const fixtureRule = (revision = 1, appId = 'app-a'): OwnerRuleRevisionDto => ({
  rule_id: 'rule-a', app_id: appId, capability_id: 'capability-a', revision,
  expected_behavior: `Staff can read a new booking, revision ${revision}`, provenance: 'owner',
  confirmed_by: 'Synthetic owner', confirmed_at: revision, supersedes: revision > 1 ? revision - 1 : null,
});
export const fixtureDraft = (appId = 'app-a', version = 1): OwnerChangeDraftDto => ({
  id: 'change-a', app_id: appId, version, intent: 'Add booking reminders', example: 'One reminder a day before',
  scope: 'Email only', preserved_rules: [{ rule_id: 'rule-a', revision: 1 }],
  candidate_id: null, state: 'draft', updated_at: version,
});
export function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void; reject: (reason: unknown) => void } {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
