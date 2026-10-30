import { describe, expect, it, vi } from 'vitest';
import type { OwnerAppDto, OwnerChangeDraftDto, OwnerChangeReviewDto } from '../tauri/bindings';
import { ChangeRequestStore, type ChangeRequestDependencies } from './change-request.svelte';
import { deferred, fixtureApp, fixtureDraft, fixtureRule } from '../../features/owner-workspace/owner-workspace-fixtures';

function dependencies(overrides: Partial<ChangeRequestDependencies> = {}): Partial<ChangeRequestDependencies> {
  let id = 0;
  return { getApp: vi.fn(async (appId) => fixtureApp(appId)),
    listRules: vi.fn(async () => ({ items: [fixtureRule()], next_cursor: null })),
    getReview: vi.fn(async (appId) => ({ app: fixtureApp(appId), change: fixtureDraft(appId), preserved_rules: [fixtureRule()], candidate: null, environment: null })),
    save: vi.fn(async (input) => ({ ...input, version: input.expected_version + 1, updated_at: 2 })), uuid: () => `synthetic-${++id}`, ...overrides };
}

describe('ChangeRequestStore', () => {
  it('saves and reopens a request with the exact preserved revisions', async () => {
    const save = vi.fn(async (input) => ({ ...input, version: 1, updated_at: 2 }));
    const store = new ChangeRequestStore(dependencies({ save }));
    await store.load('app-a');
    store.intent = 'Remind the customer'; store.example = 'One email a day before';
    expect(store.dirty).toBe(true);
    expect(await store.save()).toBe(true);
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ expected_version: 0, preserved_rules: [{ rule_id: 'rule-a', revision: 1 }], app_id: 'app-a' }));
    expect(store.dirty).toBe(false);
    await store.load('app-a', 'change-a');
    expect(store.intent).toBe('Add booking reminders');
    expect(store.preservedRules).toEqual([{ rule_id: 'rule-a', revision: 1 }]);
  });

  it('starts two successive changes from current confirmed rules without copying evidence', async () => {
    const store = new ChangeRequestStore(dependencies({ listRules: vi.fn().mockResolvedValueOnce({ items: [fixtureRule()], next_cursor: null }).mockResolvedValueOnce({ items: [fixtureRule(), fixtureRule(2)], next_cursor: null }) }));
    await store.load('app-a'); const firstId = store.changeId;
    store.intent = 'First change'; store.example = 'One example'; await store.save();
    await store.load('app-a');
    expect(store.changeId).not.toBe(firstId);
    expect(store.preservedRules).toEqual([{ rule_id: 'rule-a', revision: 2 }]);
    expect(store.intent).toBe('');
    expect(store.draft).toBeNull();
  });

  it('requires review of a superseded preserved rule and keeps the old revision until explicit choice', async () => {
    const store = new ChangeRequestStore(dependencies({ listRules: vi.fn(async () => ({ items: [fixtureRule(), fixtureRule(2)], next_cursor: null })) }));
    await store.load('app-a', 'change-a');
    expect(store.outdatedRules).toEqual([{ rule_id: 'rule-a', revision: 1 }]);
    expect(store.canReview).toBe(false);
    expect(await store.save('ready_for_review')).toBe(false);
    store.useLatestRules();
    expect(store.preservedRules[0]?.revision).toBe(2);
    expect(store.canReview).toBe(true);
    expect(store.dirty).toBe(true);
  });

  it('keeps wording on conflict and creates a separate draft without overwriting', async () => {
    const save = vi.fn().mockRejectedValue({ code: 'owner_conflict', message: 'The saved version changed.', retryable: false });
    const store = new ChangeRequestStore(dependencies({ save }));
    await store.load('app-a', 'change-a'); store.intent = 'Keep this new wording';
    expect(await store.save()).toBe(false);
    expect(store.conflict).toBe(true);
    expect(store.intent).toBe('Keep this new wording');
    expect(await store.save()).toBe(false);
    expect(save).toHaveBeenCalledTimes(1);
    store.saveAsNew();
    expect(store.changeId).not.toBe('change-a');
    expect(store.draft).toBeNull();
    expect(store.conflict).toBe(false);
    expect(store.intent).toBe('Keep this new wording');
  });

  it('discards cross-app loads and save responses', async () => {
    const lateApp = deferred<OwnerAppDto>();
    const lateSave = deferred<OwnerChangeDraftDto>();
    const store = new ChangeRequestStore(dependencies({ getApp: vi.fn((id) => id === 'slow' ? lateApp.promise : Promise.resolve(fixtureApp(id))), save: vi.fn(() => lateSave.promise) }));
    const pendingLoad = store.load('slow');
    await store.load('app-a'); lateApp.resolve(fixtureApp('slow')); await pendingLoad;
    expect(store.context?.app.id).toBe('app-a');
    store.intent = 'First app edit'; const pendingSave = store.save();
    await store.load('app-b'); lateSave.resolve(fixtureDraft('app-a')); expect(await pendingSave).toBe(false);
    expect(store.context?.app.id).toBe('app-b'); expect(store.draft).toBeNull(); expect(store.intent).toBe('');
  });

  it('leaves edits made during a save dirty', async () => {
    const pending = deferred<OwnerChangeDraftDto>();
    const store = new ChangeRequestStore(dependencies({ save: vi.fn(() => pending.promise) }));
    await store.load('app-a'); store.intent = 'Submitted wording';
    const saving = store.save(); store.intent = 'Newer wording';
    pending.resolve({ ...fixtureDraft(), id: store.changeId! }); await saving;
    expect(store.intent).toBe('Newer wording'); expect(store.dirty).toBe(true);
  });

  it('clears the prior app immediately and reports load errors without exposing it', async () => {
    const failed = deferred<OwnerChangeReviewDto>();
    const store = new ChangeRequestStore(dependencies({ getReview: vi.fn(() => failed.promise) }));
    await store.load('app-a'); store.intent = 'Private to first app';
    const load = store.load('app-b', 'change-b');
    expect(store.context).toBeNull(); expect(store.intent).toBe('');
    failed.reject({ code: 'database', message: 'Could not read this request.', retryable: true }); await load;
    expect(store.state.status).toBe('error'); expect(store.context).toBeNull();
  });
});
