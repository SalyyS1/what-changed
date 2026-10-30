import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import ManualResultEditor from './ManualResultEditor.svelte';
import OwnerDecisionDialog from './OwnerDecisionDialog.svelte';
import { fixtureEvidenceReview, fixtureScenario } from './owner-evidence-fixtures';

describe('manual observation and owner decision', () => {
  it('starts every assertion as not run and records only explicit observation outcomes', async () => {
    const record = vi.fn().mockResolvedValue(true);
    render(ManualResultEditor, { scenario: fixtureScenario(), onRecord: record });
    expect(screen.getByRole('button', { name: /Outcome for/ }).textContent?.trim()).toBe('Not run');
    await fireEvent.input(screen.getByLabelText('Observer'), { target: { value: 'Test owner' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Record this observation' }));
    expect(record).toHaveBeenCalledWith(expect.objectContaining({ observer: 'Test owner', assertions: [expect.objectContaining({ status: 'not_run', observed: '' })] }));
  });

  it('requires reason, unknown-build and every named gap acknowledgment for exceptions', async () => {
    const record = vi.fn().mockResolvedValue(true);
    const review = fixtureEvidenceReview(); review.run.lifecycle = 'completed'; review.can_accept_with_exceptions = true; review.decision_blockers = [];
    render(OwnerDecisionDialog, { open: true, review, onRecord: record });
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Owner decision' }), { key: 'Enter' });
    await fireEvent.pointerUp(await screen.findByRole('option', { name: 'Accept with explicit exceptions' }), { pointerType: 'mouse' });
    const button = screen.getByRole('button', { name: 'Record owner decision' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    await fireEvent.input(screen.getByLabelText('Reason for this decision'), { target: { value: 'I accept the named incomplete check for this prototype.' } });
    await fireEvent.click(screen.getByLabelText(/tested build is not independently established/));
    expect(button.disabled).toBe(true);
    await fireEvent.click(screen.getByLabelText(/I accept this explicit gap/));
    expect(button.disabled).toBe(false);
    await fireEvent.click(button);
    expect(record).toHaveBeenCalledWith(expect.objectContaining({ decision: 'accepted_with_exceptions', acknowledge_unknown_build: true, acknowledged_unresolved: [{ scenario_id: 'scenario-a', revision: 1 }] }));
    expect(review.status).toBe('not_run');
  });

  it('accepts named current-build gaps without requiring an untrue unknown-build acknowledgment', async () => {
    const record = vi.fn().mockResolvedValue(true);
    const review = fixtureEvidenceReview(); review.run.lifecycle = 'completed'; review.run.method = 'automated';
    review.freshness = 'current'; review.status = 'failed'; review.can_accept_with_exceptions = true; review.decision_blockers = [];
    render(OwnerDecisionDialog, { open: true, review, onRecord: record });
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Owner decision' }), { key: 'Enter' });
    await fireEvent.pointerUp(await screen.findByRole('option', { name: 'Accept with explicit exceptions' }), { pointerType: 'mouse' });
    expect(screen.queryByLabelText(/tested build is not independently established/)).toBeNull();
    await fireEvent.input(screen.getByLabelText('Reason for this decision'), { target: { value: 'Accept this specific current-build limitation.' } });
    await fireEvent.click(screen.getByLabelText(/I accept this explicit gap/));
    await fireEvent.click(screen.getByRole('button', { name: 'Record owner decision' }));
    expect(record).toHaveBeenCalledWith(expect.objectContaining({ decision: 'accepted_with_exceptions', acknowledge_unknown_build: false, acknowledged_unresolved: [{ scenario_id: 'scenario-a', revision: 1 }] }));
    expect(review.status).toBe('failed');
  });
});
