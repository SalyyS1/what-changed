import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import ScenarioEditor from './ScenarioEditor.svelte';
import { fixtureRule } from '../owner-workspace/owner-workspace-fixtures';
import { fixtureScenario } from './owner-evidence-fixtures';

const capabilities = [{ id: 'capability-a', app_id: 'app-a', label: 'Bookings', description: '', sort_order: 0, version: 1, archived_at: null }];
describe('ScenarioEditor', () => {
  it('keeps a template unconfirmed until the owner reviews and explicitly confirms it', async () => {
    const confirm = vi.fn().mockResolvedValue(true);
    render(ScenarioEditor, { capabilities, rules: [fixtureRule()], scenarios: [], origins: [], onConfirm: confirm });
    await fireEvent.click(screen.getByRole('button', { name: 'Booking creation template' }));
    expect(confirm).not.toHaveBeenCalled();
    await fireEvent.input(screen.getByLabelText('Confirmed by'), { target: { value: 'Test owner' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Review revision' }));
    expect(await screen.findByRole('dialog', { name: 'Confirm this scenario revision?' })).toBeTruthy();
    const button = screen.getByRole('button', { name: 'Confirm revision' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    await fireEvent.click(screen.getByLabelText(/I reviewed the old and proposed behavior/));
    await fireEvent.click(button);
    expect(confirm).toHaveBeenCalledWith(expect.objectContaining({ method: 'owner_manual', confirmed_by: 'Test owner', rules: [{ rule_id: 'rule-a', revision: 1 }] }), undefined);
  });

  it('shows old and proposed expected behavior before confirming a revision', async () => {
    render(ScenarioEditor, { capabilities, rules: [fixtureRule()], scenarios: [fixtureScenario()], origins: [], onConfirm: vi.fn() });
    await fireEvent.click(screen.getByRole('button', { name: 'Propose a revision' }));
    await fireEvent.input(screen.getByLabelText('Expected business behavior'), { target: { value: 'New explicit expectation' } });
    await fireEvent.input(screen.getByLabelText('Confirmed by'), { target: { value: 'Test owner' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Review revision' }));
    expect(await screen.findByText('Current confirmed revision 1')).toBeTruthy();
    expect(screen.getByText('New explicit expectation')).toBeTruthy();
  });
});
