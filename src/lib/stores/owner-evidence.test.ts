import { describe, expect, it, vi } from 'vitest';
import type { OwnerEvidenceReviewDto, OwnerManualResultDto } from '../tauri/bindings';
import { OwnerEvidenceStore, type OwnerEvidenceDependencies } from './owner-evidence.svelte';
import { deferred } from '../../features/owner-workspace/owner-workspace-fixtures';
import { fixtureCandidate, fixtureChangeReview, fixtureEvidenceReview, fixtureManualRun, fixtureScenario } from '../../features/acceptance/owner-evidence-fixtures';

function dependencies(overrides: Partial<OwnerEvidenceDependencies> = {}): Partial<OwnerEvidenceDependencies> {
  let id = 0;
  return { getChange: vi.fn(async (appId) => fixtureChangeReview(appId)), listScenarios: vi.fn(async () => ({ items: [fixtureScenario()], next_cursor: null })),
    listCandidates: vi.fn(async () => ({ items: [fixtureCandidate()], next_cursor: null })), listRuns: vi.fn(async () => ({ items: [fixtureEvidenceReview()], next_cursor: null })),
    listDecisions: vi.fn(async () => ({ items: [], next_cursor: null })), getRun: vi.fn(async (appId) => fixtureEvidenceReview(appId)),
    start: vi.fn(async () => fixtureManualRun()), confirmScenario: vi.fn(async () => fixtureScenario(2)),
    saveChange: vi.fn(async (input) => ({ ...input, version: input.expected_version + 1, updated_at: 2 })),
    uuid: () => `synthetic-${++id}`, ...overrides };
}

describe('OwnerEvidenceStore', () => {
  it('freezes explicit candidate and scenario versions without starting a runner', async () => {
    const start = vi.fn(async () => fixtureManualRun());
    const store = new OwnerEvidenceStore(dependencies({ start }));
    await store.load('app-a', 'change-a');
    expect(await store.start([{ scenario_id: 'scenario-a', revision: 1 }])).toBe(true);
    expect(start).toHaveBeenCalledWith(expect.objectContaining({ app_id: 'app-a', change_id: 'change-a', expected_change_version: 1, candidate_id: 'candidate-a', scenarios: [{ scenario_id: 'scenario-a', revision: 1 }], retry_of: null }));
    expect(store.currentReview?.freshness).toBe('unknown'); expect(store.currentReview?.can_accept).toBe(false);
  });

  it('binds the candidate only after an explicit saved-request update', async () => {
    const saveChange = vi.fn(async (input) => ({ ...input, version: 2, updated_at: 2 }));
    const review = fixtureChangeReview(); review.change.candidate_id = null;
    const store = new OwnerEvidenceStore(dependencies({ getChange: vi.fn(async () => review), saveChange }));
    await store.load('app-a', 'change-a'); expect(saveChange).not.toHaveBeenCalled();
    await store.bindSelectedCandidate();
    expect(saveChange).toHaveBeenCalledWith(expect.objectContaining({ expected_version: 1, candidate_id: 'candidate-a', intent: review.change.intent }));
  });

  it('keeps backend status, gaps and freshness even when owner decisions differ', async () => {
    const review = fixtureEvidenceReview(); review.status = 'failed'; review.counts = { passed: 0, failed: 1, blocked: 0, not_run: 0 };
    const store = new OwnerEvidenceStore(dependencies({ getRun: vi.fn(async () => review) }));
    await store.load('app-a', 'change-a'); await store.selectRun('run-a');
    expect(store.currentReview?.status).toBe('failed'); expect(store.currentReview?.freshness).toBe('unknown');
    expect(store.currentReview?.can_accept).toBe(false); expect(store.currentReview?.unresolved_scenarios).toEqual(review.unresolved_scenarios);
  });

  it('discards a late result save after the owner switches apps', async () => {
    const pending = deferred<OwnerManualResultDto>(); const getRun = vi.fn(async (appId) => fixtureEvidenceReview(appId));
    const store = new OwnerEvidenceStore(dependencies({ record: vi.fn(() => pending.promise), getRun }));
    await store.load('app-a', 'change-a'); await store.selectRun('run-a');
    const saving = store.record({ scenario_id: 'scenario-a', scenario_revision: 1, observer: 'Owner', observed_at: 1, observed_steps: [], assertions: [], limitation: 'Not performed' });
    await store.load('app-b', 'change-b');
    pending.resolve({ id: 'result-a', app_id: 'app-a', run_id: 'run-a', scenario_id: 'scenario-a', scenario_revision: 1, method: 'owner_manual', status: 'not_run', observer: 'Owner', observed_at: 1, observed_steps: [], assertions: [], limitation: 'Not performed', recorded_at: 1 });
    expect(await saving).toBe(false); expect(store.appId).toBe('app-b'); expect(store.currentReview).toBeNull();
    expect(getRun).toHaveBeenCalledTimes(1);
  });

  it('discards a late review when a different run is selected', async () => {
    const pending = deferred<OwnerEvidenceReviewDto>();
    const store = new OwnerEvidenceStore(dependencies({ getRun: vi.fn((_, id) => id === 'slow' ? pending.promise : Promise.resolve({ ...fixtureEvidenceReview(), run: { ...fixtureManualRun(), id } })) }));
    await store.load('app-a', 'change-a'); const slow = store.selectRun('slow'); await store.selectRun('new');
    pending.resolve({ ...fixtureEvidenceReview(), run: { ...fixtureManualRun(), id: 'slow' } }); await slow;
    expect(store.currentReview?.run.id).toBe('new');
  });

  it('keeps expected versions and owner exception acknowledgments in a decision', async () => {
    const decide = vi.fn().mockResolvedValue({});
    const store = new OwnerEvidenceStore(dependencies({ decide }));
    await store.load('app-a', 'change-a'); await store.selectRun('run-a');
    await store.decide({ decision: 'accepted_with_exceptions', reason: 'Owner accepts a named gap', acknowledged_unresolved: [{ scenario_id: 'scenario-a', revision: 1 }], acknowledge_unknown_build: true });
    expect(decide).toHaveBeenCalledWith(expect.objectContaining({ expected_run_version: 1, expected_change_version: 1, acknowledged_unresolved: [{ scenario_id: 'scenario-a', revision: 1 }], acknowledge_unknown_build: true }));
    expect(store.currentReview?.status).toBe('not_run');
  });
});
