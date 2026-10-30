import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import RunConsent from './RunConsent.svelte';
import { fixtureManualRun } from '../acceptance/owner-evidence-fixtures';

describe('one-session preview permission', () => {
  it('requires all three acknowledgments and exposes the frozen scope before granting', async () => {
    const onGrant = vi.fn();
    render(RunConsent, { open: true, onGrant, plan: { consent_id: 'consent-a', scope_digest: 'scope-a', manifest_digest: 'manifest-a', manifest: fixtureManualRun().manifest, roles: ['customer-test'], routes: [{ origin: 'http://127.0.0.1:3000', method: 'POST', path: '/api/bookings' }], effects: ['May create a test booking.'], limitations: ['Unknown build remains unknown.'], capture_screenshots: false } });
    const grant = screen.getByRole('button', { name: 'Allow this session' }) as HTMLButtonElement;
    expect(grant.disabled).toBe(true); expect(screen.getByText(/POST http:\/\/127.0.0.1:3000\/api\/bookings/)).toBeTruthy();
    await fireEvent.click(screen.getByLabelText(/I own this test environment/));
    await fireEvent.click(screen.getByLabelText(/This uses isolated test accounts/));
    expect(grant.disabled).toBe(true); expect(onGrant).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByLabelText(/temporary profiles may contain/));
    expect(grant.disabled).toBe(false); await fireEvent.click(grant); expect(onGrant).toHaveBeenCalledOnce();
  });
});
