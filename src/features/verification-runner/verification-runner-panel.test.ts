import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import VerificationRunnerPanel from './VerificationRunnerPanel.svelte';
import { PreviewRunnerStore } from '../../lib/stores/preview-runner.svelte';
import { fixtureChangeReview, fixtureManualRun, fixtureScenario } from '../acceptance/owner-evidence-fixtures';
import type { PreviewRunPlanDto, PreviewRunnerStatusDto } from '../../lib/tauri/bindings';

const plan = (): PreviewRunPlanDto => ({ consent_id: 'consent-a', manifest: fixtureManualRun().manifest, manifest_digest: 'digest-a', scope_digest: 'scope-a', roles: ['customer-test'], routes: [], effects: [], limitations: [], capture_screenshots: false });
const session = (): PreviewRunnerStatusDto => ({ app_id: 'app-a', run_id: 'run-a', lifecycle: 'running', roles: ['customer-test'], can_continue: false, mode: 'run', events: [], error: null, capture_screenshots: false, captures: [] });

describe('preview panel integration', () => {
  it('does not prepare on mount and requires reviewing before granting a session', async () => {
    const prepare = vi.fn(async () => plan()); const grant = vi.fn(async () => session()); const cancel = vi.fn(async () => {});
    const model = new PreviewRunnerStore({ prepare, grant, cancel });
    const view = render(VerificationRunnerPanel, { change: fixtureChangeReview(), scenarios: [fixtureScenario()], model });
    expect(prepare).not.toHaveBeenCalled(); expect(grant).not.toHaveBeenCalled();
    await fireEvent.input(screen.getByLabelText('Synthetic value for booking-test'), { target: { value: 'Synthetic booking' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Review test session' }));
    await screen.findByRole('dialog', { name: 'Allow this test session?' });
    expect(prepare).toHaveBeenCalledOnce(); expect(grant).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByLabelText(/I own this test environment/));
    await fireEvent.click(screen.getByLabelText(/This uses isolated test accounts/));
    await fireEvent.click(screen.getByLabelText(/temporary profiles may contain/));
    await fireEvent.click(screen.getByRole('button', { name: 'Allow this session' }));
    await waitFor(() => expect(grant).toHaveBeenCalledOnce());
    await screen.findByRole('button', { name: 'Revoke permission and stop' });
    view.unmount(); await waitFor(() => expect(cancel).toHaveBeenCalledWith('app-a', 'run-a'));
  });

  it('invalidates a reviewed scope when saved request version changes', async () => {
    const model = new PreviewRunnerStore({ prepare: async () => plan() });
    const original = fixtureChangeReview();
    const view = render(VerificationRunnerPanel, { change: original, scenarios: [fixtureScenario()], model });
    await fireEvent.input(screen.getByLabelText('Synthetic value for booking-test'), { target: { value: 'Synthetic booking' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Review test session' }));
    await screen.findByRole('dialog', { name: 'Allow this test session?' });
    const changed = structuredClone(original); changed.change.version += 1;
    await view.rerender({ change: changed, scenarios: [fixtureScenario()], model });
    await waitFor(() => expect(model.plan).toBeNull());
    expect(screen.queryByRole('dialog', { name: 'Allow this test session?' })).toBeNull();
  });
});
