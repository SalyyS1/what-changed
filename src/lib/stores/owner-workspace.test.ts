import { describe, expect, it, vi } from 'vitest';
import type { OwnerAppDto, OwnerCapabilityDto, OwnerRuleRevisionDto } from '../tauri/bindings';
import { OwnerWorkspaceStore, ownerPages, type OwnerWorkspaceDependencies } from './owner-workspace.svelte';
import { deferred, fixtureApp, fixtureRule } from '../../features/owner-workspace/owner-workspace-fixtures';

const page = <T>(items: T[]) => Promise.resolve({ items, next_cursor: null });
function dependencies(overrides: Partial<OwnerWorkspaceDependencies> = {}): Partial<OwnerWorkspaceDependencies> {
  return { listApps: vi.fn().mockResolvedValue([]), getApp: vi.fn(async (id) => fixtureApp(id)),
    listCapabilities: vi.fn(async () => []), listRules: vi.fn(async () => ({ items: [fixtureRule()], next_cursor: null })),
    listChanges: vi.fn(() => page([])), listEnvironments: vi.fn(() => page([])), listCandidates: vi.fn(() => page([])),
    uuid: () => 'synthetic-operation-id', ...overrides };
}

describe('OwnerWorkspaceStore', () => {
  it('discards an app load that finishes after the owner switched apps', async () => {
    const first = deferred<OwnerAppDto>();
    const store = new OwnerWorkspaceStore(dependencies({ getApp: vi.fn((id) => id === 'app-a' ? first.promise : Promise.resolve(fixtureApp(id))) }));
    const stale = store.loadApp('app-a');
    await store.loadApp('app-b');
    first.resolve(fixtureApp('app-a'));
    await stale;
    expect(store.current?.app.id).toBe('app-b');
  });

  it('does not refresh or display a late mutation in a different app', async () => {
    const pending = deferred<OwnerCapabilityDto>();
    const getApp = vi.fn(async (id: string) => fixtureApp(id));
    const saveCapability = vi.fn(() => pending.promise);
    const store = new OwnerWorkspaceStore(dependencies({ getApp, saveCapability }));
    await store.loadApp('app-a');
    const saving = store.saveCapability({ label: 'Bookings', description: 'Create one' });
    await store.loadApp('app-b');
    pending.resolve({ id: 'capability-a', app_id: 'app-a', label: 'Bookings', description: 'Create one', version: 1, sort_order: 0, archived_at: null });
    expect(await saving).toBe(false);
    expect(store.current?.app.id).toBe('app-b');
    expect(getApp).toHaveBeenCalledTimes(2);
    expect(store.saving).toBe(false);
  });

  it('uses explicit UUID operations and authoritative versions for create and archive', async () => {
    let next = 0;
    const saved = fixtureApp();
    const saveApp = vi.fn().mockResolvedValueOnce(saved).mockResolvedValueOnce({ ...saved, version: 2, archived_at: 10 });
    const store = new OwnerWorkspaceStore(dependencies({ saveApp, uuid: () => `uuid-${++next}` }));
    await store.loadApps();
    await store.saveApp({ name: saved.name, description: '', legacy_project_id: null, archived: false });
    await store.saveApp({ name: saved.name, description: '', legacy_project_id: null, archived: true }, saved);
    expect(saveApp.mock.calls[0]?.[0]).toMatchObject({ operation_id: 'uuid-1', id: 'uuid-2', expected_version: 0 });
    expect(saveApp.mock.calls[1]?.[0]).toMatchObject({ expected_version: 1, archived: true });
    expect(store.apps.status).toBe('empty');
  });

  it('keeps revisions and provenance when confirming an existing rule', async () => {
    const confirmRule = vi.fn(async () => fixtureRule(2));
    const store = new OwnerWorkspaceStore(dependencies({ confirmRule }));
    await store.loadApp('app-a');
    expect(await store.confirmRule('capability-a', 'Revised expectation', fixtureRule())).toBe(true);
    expect(confirmRule).toHaveBeenCalledWith(expect.objectContaining({ expected_version: 1, rule_id: 'rule-a', app_id: 'app-a', provenance: 'owner', expected_behavior: 'Revised expectation' }));
  });

  it('loads all pages so a later request can reuse every confirmed rule', async () => {
    const read = vi.fn().mockResolvedValueOnce({ items: [fixtureRule()], next_cursor: 'next' }).mockResolvedValueOnce({ items: [fixtureRule(2)], next_cursor: null });
    const rules = await ownerPages<OwnerRuleRevisionDto>(read);
    expect(rules.map((rule) => rule.revision)).toEqual([1, 2]);
    expect(read).toHaveBeenLastCalledWith({ after: 'next', limit: 100 });
  });
});
