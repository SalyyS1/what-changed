import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import AdapterEditor from './AdapterEditor.svelte';
import RecipeStepEditor from './RecipeStepEditor.svelte';
import ScenarioEditor from './ScenarioEditor.svelte';
import { scenarioTemplate } from './scenario-recipes';
import { fixtureRule } from '../owner-workspace/owner-workspace-fixtures';

const origin = { scheme: 'http', host: 'localhost', port: 3000 };
describe('reviewed route editing', () => {
  it('deduplicates exact origin options before the keyed Select opens', async () => {
    const recipe = scenarioTemplate('booking-read-back', origin)!.recipe;
    render(AdapterEditor, { adapters: recipe.adapters, origins: [origin, { ...origin }] });
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Route 1 exact preview origin' }), { key: 'Enter' });
    expect(await screen.findAllByRole('option')).toHaveLength(1);
  });
  it('offers only compatible routes and enforces the aggregate add-check limit', async () => {
    const recipe = scenarioTemplate('booking-read-back', origin)!.recipe; const read = recipe.adapters[0]!;
    render(RecipeStepEditor, { step: recipe.steps[0]!, index: 0, adapters: [{ ...read, id: 'denied-route', effect: 'read_denied' }, read], totalAssertions: 1000 });
    expect((screen.getByRole('button', { name: 'Add expected check' }) as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Reviewed read-back route' }), { key: 'Enter' });
    expect(await screen.findByRole('option', { name: 'Route 2 · staff-test' })).toBeTruthy();
    expect(screen.queryByRole('option', { name: 'Route 1 · staff-test' })).toBeNull();
  });
  it('blocks review after role or allowed-field edits until the owner corrects the mismatch', async () => {
    const confirm = vi.fn(); const capabilities = [{ id: 'capability-a', app_id: 'app-a', label: 'Bookings', description: '', sort_order: 0, version: 1, archived_at: null }];
    render(ScenarioEditor, { capabilities, rules: [fixtureRule()], scenarios: [], origins: [origin], onConfirm: confirm });
    await fireEvent.click(screen.getByRole('button', { name: 'Stored booking template' }));
    await fireEvent.input(screen.getByLabelText('Confirmed by'), { target: { value: 'Owner' } });
    await fireEvent.input(screen.getByLabelText('Test-role references (comma separated, no credentials)'), { target: { value: 'customer-test' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Review revision' }));
    expect(screen.queryByRole('dialog')).toBeNull(); expect(screen.getByText(/its test role must exactly match/)).toBeTruthy();
    await fireEvent.input(screen.getByLabelText('Test-role references (comma separated, no credentials)'), { target: { value: 'staff-test' } });
    await fireEvent.input(screen.getByLabelText('Allowed response fields (comma separated)'), { target: { value: 'customer_name' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Review revision' }));
    expect(screen.queryByRole('dialog')).toBeNull(); expect(screen.getByText(/stored field must be selected/)).toBeTruthy(); expect(confirm).not.toHaveBeenCalled();
  });
});
