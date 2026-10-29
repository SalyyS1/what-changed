import { describe, expect, it, vi } from 'vitest';
import type { PreparePreviewRunInput, PreviewRunPlanDto, PreviewRunnerStatusDto } from '../tauri/bindings';
import { PreviewRunnerStore } from './preview-runner.svelte';
import { PreviewRevocationsStore, previewRevocations } from './preview-revocations.svelte';
import { fixtureManualRun } from '../../features/acceptance/owner-evidence-fixtures';
import { deferred } from '../../features/owner-workspace/owner-workspace-fixtures';

function request(): PreparePreviewRunInput {
  return { run: { operation_id: 'operation-a', id: 'run-a', app_id: 'app-a', change_id: 'change-a', expected_change_version: 1, candidate_id: 'candidate-a', scenarios: [{ scenario_id: 'scenario-a', revision: 1 }], retry_of: null }, routes: [], fixture: { customer: 'Synthetic customer' }, mode: 'run', capture_screenshots: false };
}
function plan(): PreviewRunPlanDto {
  return { consent_id: 'consent-a', manifest: fixtureManualRun().manifest, manifest_digest: 'manifest-a', scope_digest: 'scope-a', roles: ['customer'], routes: [], effects: [], limitations: [], capture_screenshots: false };
}
function session(lifecycle = 'running'): PreviewRunnerStatusDto {
  return { app_id: 'app-a', run_id: 'run-a', lifecycle, roles: ['customer'], can_continue: false, mode: 'run', events: [], error: null, captures: [], capture_screenshots: false };
}

describe('PreviewRunnerStore authority and lifecycle', () => {
  it('prepares a frozen scope without granting or starting execution', async () => {
    const prepare = vi.fn(async (_input: PreparePreviewRunInput) => plan()); const grant = vi.fn();
    const store = new PreviewRunnerStore({ prepare, grant });
    const input = request(); await store.prepare(input); input.fixture.customer = 'Changed';
    expect(grant).not.toHaveBeenCalled(); expect(store.session).toBeNull();
    expect(prepare.mock.calls[0]?.[0].fixture.customer).toBe('Synthetic customer');
    expect(store.plan?.scope_digest).toBe('scope-a');
  });

  it('grants exactly the reviewed consent and scope with explicit acknowledgments', async () => {
    const grant = vi.fn(async () => session());
    const store = new PreviewRunnerStore({ prepare: async () => plan(), grant });
    expect(await store.grant()).toBeNull(); await store.prepare(request()); await store.grant();
    expect(grant).toHaveBeenCalledExactlyOnceWith({ consent_id: 'consent-a', scope_digest: 'scope-a', authorized_test_environment: true, isolated_test_data: true, acknowledge_temporary_auth: true });
    expect(store.running).toBe(true); expect(store.plan).toBeNull();
  });

  it('revokes a late native grant after navigation', async () => {
    const pending = deferred<PreviewRunnerStatusDto>(); const cancel = vi.fn(async () => {});
    const store = new PreviewRunnerStore({ prepare: async () => plan(), grant: () => pending.promise, cancel });
    await store.prepare(request()); const granted = store.grant(); store.dispose(); pending.resolve(session());
    expect(await granted).toBeNull(); expect(cancel).toHaveBeenCalledWith('app-a', 'run-a'); expect(store.session).toBeNull();
  });

  it('keeps cancellation recovery after navigation clears the workspace', async () => {
    const revocations = new PreviewRevocationsStore();
    const cancel = vi.fn(async () => {}).mockRejectedValueOnce({ code: 'not_ready', message: 'Cancellation unavailable', retryable: true });
    const status = vi.fn(async () => session('cancelled')).mockRejectedValueOnce({ code: 'not_ready', message: 'Status unavailable', retryable: true });
    const store = new PreviewRunnerStore({ prepare: async () => plan(), grant: async () => session(), cancel, status }, revocations);
    await store.prepare(request()); await store.grant(); store.dispose();
    await revocations.retry('app-a', 'run-a');
    expect(store.session).toBeNull();
    expect(revocations.sessions).toMatchObject([{ app_id: 'app-a', run_id: 'run-a', busy: false }]);
    expect(revocations.sessions[0]?.error).toContain('Cancellation unavailable');
    await revocations.retry('app-a', 'run-a');
    expect(cancel).toHaveBeenNthCalledWith(2, 'app-a', 'run-a');
    expect(revocations.sessions).toEqual([]);
  });

  it('retains a failed late-grant revocation independently of the next workspace generation', async () => {
    const pending = deferred<PreviewRunnerStatusDto>(); const revocations = new PreviewRevocationsStore();
    const cancel = vi.fn(async () => {}).mockRejectedValueOnce({ code: 'not_ready', message: 'Late cancellation unavailable', retryable: true });
    const status = vi.fn(async () => session('cancelled')).mockRejectedValueOnce({ code: 'not_ready', message: 'Status unavailable', retryable: true });
    const store = new PreviewRunnerStore({ prepare: async () => plan(), grant: () => pending.promise, cancel, status }, revocations);
    await store.prepare(request()); const granted = store.grant(); store.dispose();
    await store.prepare(request()); pending.resolve(session());
    expect(await granted).toBeNull();
    await revocations.retry('app-a', 'run-a');
    expect(store.session).toBeNull(); expect(store.plan?.consent_id).toBe('consent-a');
    expect(revocations.sessions).toMatchObject([{ app_id: 'app-a', run_id: 'run-a' }]);
    expect(revocations.sessions[0]?.error).toContain('Late cancellation unavailable');
    await revocations.retry('app-a', 'run-a');
    expect(revocations.sessions).toEqual([]); expect(store.plan?.consent_id).toBe('consent-a');
  });

  it('isolates injected runner recovery from the default app owner and other test adapters', () => {
    const first = new PreviewRunnerStore({ cancel: vi.fn(async () => {}) });
    const second = new PreviewRunnerStore({ cancel: vi.fn(async () => {}) });
    expect(first.revocations).not.toBe(previewRevocations);
    expect(first.revocations).not.toBe(second.revocations);
    expect(new PreviewRunnerStore().revocations).toBe(previewRevocations);
  });

  it('keeps an unknown read failure distinct from completion and allows cancellation', async () => {
    const cancel = vi.fn(async () => {}); const status = vi.fn().mockRejectedValue({ code: 'not_ready', message: 'Status unavailable', retryable: true });
    const store = new PreviewRunnerStore({ prepare: async () => plan(), grant: async () => session(), status, cancel });
    await store.prepare(request()); await store.grant(); await store.refresh();
    expect(store.running).toBe(true); expect(store.error).toContain('Execution status is unknown');
    await store.cancel(); expect(cancel).toHaveBeenCalledWith('app-a', 'run-a');
  });

  it('retains native terminal status without converting failures into a pass or owner acceptance', async () => {
    const store = new PreviewRunnerStore({ prepare: async () => plan(), grant: async () => session(), status: async () => ({ ...session('interrupted'), error: 'Revision changed' }) });
    await store.prepare(request()); await store.grant(); await store.refresh();
    expect(store.session?.lifecycle).toBe('interrupted'); expect(store.session?.error).toBe('Revision changed'); expect(store.running).toBe(false);
  });

  it('does not execute exported recipes and drops late scope previews', async () => {
    const pending = deferred<PreviewRunPlanDto>(); const exportRecipe = vi.fn(); const grant = vi.fn();
    const store = new PreviewRunnerStore({ prepare: () => pending.promise, grant, exportRecipe });
    const prepared = store.prepare(request()); store.dispose(); pending.resolve(plan());
    expect(await prepared).toBe(false); expect(store.plan).toBeNull();
    await store.previewExport(); expect(exportRecipe).not.toHaveBeenCalled(); expect(grant).not.toHaveBeenCalled();
  });
});
