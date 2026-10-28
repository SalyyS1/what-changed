import { describe, expect, it } from 'vitest';
import type { ScenarioRecipeDto } from '../../lib/tauri/bindings';
import { scenarioTemplate } from './scenario-recipes';
import { actionAdapters, assertionCount, candidateOrigins, canAddAssertion, recipeCompatibilityIssues } from './scenario-validation';
import { fixtureCandidate, fixtureEnvironment } from './owner-evidence-fixtures';

const origin = { scheme: 'http', host: 'localhost', port: 3000 };
const recipe = (): ScenarioRecipeDto => scenarioTemplate('booking-read-back', origin)!.recipe;

describe('candidate environment for scenario routes', () => {
  it('uses only the selected candidate exact environment and deduplicates its origin tuples', () => {
    const first = fixtureEnvironment(); const second = { ...fixtureEnvironment(), id: 'environment-b', origins: [origin, origin, { ...origin, port: 4000 }] };
    const candidate = { ...fixtureCandidate(), environment_id: second.id };
    const result = candidateOrigins([first, second], candidate);
    expect(result.origins).toEqual([origin, { ...origin, port: 4000 }]); expect(result.message).toBeNull();
    expect(candidateOrigins([{ ...first, origins: [{ ...origin, port: 6000 }] }, second], candidate).origins).not.toContainEqual({ ...origin, port: 6000 });
  });
  it('blocks missing, stale and revoked selected preview revisions without fallback to another environment', () => {
    const environment = fixtureEnvironment(); const candidate = fixtureCandidate();
    expect(candidateOrigins([environment], null).origins).toEqual([]);
    expect(candidateOrigins([{ ...environment, id: 'unrelated' }], candidate).message).toContain('unavailable');
    expect(candidateOrigins([environment, { ...environment, revision: 2 }], candidate).message).toContain('older preview revision');
    expect(candidateOrigins([{ ...environment, revoked: true }], candidate).message).toContain('revoked');
  });
});

describe('scenario editor compatibility guidance', () => {
  it('filters route purpose and retains incompatible expectations for explicit correction', () => {
    const value = recipe(); const read = value.adapters[0]!; const denied = { ...read, id: 'denied-route', effect: 'read_denied' as const };
    expect(actionAdapters('read_back', [denied, read])).toEqual([read]); expect(actionAdapters('deny_other_role', [read, denied])).toEqual([denied]);
    const original = structuredClone(value.steps[0]!.assertions); value.steps[0]!.action = { kind: 'deny_other_role', adapter_id: denied.id }; value.adapters.push(denied);
    expect(recipeCompatibilityIssues(value, [origin]).some((issue) => issue.includes('stored-field comparison needs a read-back'))).toBe(true);
    expect(value.steps[0]!.assertions).toEqual(original);
  });
  it('explains missing roles, deleted routes and fields removed from the approved response list', () => {
    const value = recipe(); value.role_refs = ['different-role'];
    expect(recipeCompatibilityIssues(value, [origin]).some((issue) => issue.includes('declared test-role'))).toBe(true);
    value.role_refs = ['staff-test']; value.adapters[0]!.allowed_fields = ['customer_name'];
    expect(recipeCompatibilityIssues(value, [origin]).some((issue) => issue.includes('stored field must be selected'))).toBe(true);
    value.adapters = []; expect(recipeCompatibilityIssues(value, [origin]).some((issue) => issue.includes('existing reviewed route'))).toBe(true);
  });
  it('requires denied checks to use a matching denied route and selected absent fields', () => {
    const value = scenarioTemplate('booking-access', origin)!.recipe;
    expect(recipeCompatibilityIssues(value, [origin])).toEqual([]);
    value.steps[0]!.assertions[0]!.assertion = { kind: 'denied', allowed_statuses: [403], absent_fields: ['unapproved_field'] };
    expect(recipeCompatibilityIssues(value, [origin]).some((issue) => issue.includes('every absent field'))).toBe(true);
    value.steps[0]!.action = { kind: 'navigate', path: '/' };
    expect(recipeCompatibilityIssues(value, [origin]).some((issue) => issue.includes('deny-other-role action'))).toBe(true);
  });
  it('allows 100 checks per step and 1000 per scenario, then blocks each next check', () => {
    const value = recipe(); const prototype = value.steps[0]!;
    const step = { ...prototype, assertions: Array.from({ length: 100 }, () => ({ ...structuredClone(prototype.assertions[0]!), id: crypto.randomUUID() })) };
    value.steps = [step]; expect(recipeCompatibilityIssues(value, [origin])).toEqual([]); expect(canAddAssertion(step, 100)).toBe(false);
    step.assertions.push({ ...structuredClone(step.assertions[0]!), id: crypto.randomUUID() }); expect(recipeCompatibilityIssues(value, [origin]).some((issue) => issue.includes('Step 1: use at most 100'))).toBe(true);
    step.assertions.pop(); value.steps = Array.from({ length: 10 }, () => ({ ...structuredClone(step), id: crypto.randomUUID(), assertions: step.assertions.map((item) => ({ ...structuredClone(item), id: crypto.randomUUID() })) }));
    expect(assertionCount(value)).toBe(1000); expect(recipeCompatibilityIssues(value, [origin])).toEqual([]); expect(canAddAssertion(prototype, 1000)).toBe(false);
    value.steps.push({ ...structuredClone(prototype), id: crypto.randomUUID(), assertions: [structuredClone(prototype.assertions[0]!)] });
    expect(recipeCompatibilityIssues(value, [origin]).some((issue) => issue.includes('at most 1000'))).toBe(true);
  });
});
