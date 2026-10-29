import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import ChangeRequestScreen from './ChangeRequestScreen.svelte';
import { ChangeRequestStore } from '../../lib/stores/change-request.svelte';
import { fixtureApp, fixtureRule } from '../owner-workspace/owner-workspace-fixtures';

function fixtureModel(): ChangeRequestStore {
  return new ChangeRequestStore({ getApp: vi.fn(async () => fixtureApp()),
    listRules: vi.fn(async () => ({ items: [fixtureRule()], next_cursor: null })),
    save: vi.fn(async (input) => ({ ...input, version: input.expected_version + 1, updated_at: 2 })),
    uuid: () => 'synthetic-request' });
}

describe('ChangeRequestScreen', () => {
  it('saves owner wording and preserved rule revisions through the keyboard-readable flow', async () => {
    const model = fixtureModel();
    render(ChangeRequestScreen, { appId: 'app-a', model });
    await screen.findByLabelText('Desired outcome');
    await fireEvent.input(screen.getByLabelText('Desired outcome'), { target: { value: 'Add booking reminders' } });
    await fireEvent.input(screen.getByLabelText('A concrete example'), { target: { value: 'One email the day before' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Choose what stays' }));
    expect(await screen.findByRole('heading', { name: 'What must stay working?' })).toBeTruthy();
    expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: 'Review brief' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Mark brief ready' }));
    await waitFor(() => expect(model.draft?.state).toBe('ready_for_review'));
    expect(model.draft?.preserved_rules).toEqual([{ rule_id: 'rule-a', revision: 1 }]);
    expect(screen.getByRole('button', { name: 'Review brief export' })).toBeTruthy();
  });

  it('asks before leaving unsaved wording and preserves it when owner keeps editing', async () => {
    const model = fixtureModel(); const leave = vi.fn();
    render(ChangeRequestScreen, { appId: 'app-a', model, onLeave: leave });
    await screen.findByLabelText('Desired outcome');
    await fireEvent.input(screen.getByLabelText('Desired outcome'), { target: { value: 'Unsaved request wording' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Back to app' }));
    expect(await screen.findByRole('dialog', { name: 'Leave with unsaved changes?' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Keep editing' }));
    expect(model.intent).toBe('Unsaved request wording'); expect(leave).not.toHaveBeenCalled();
  });
});
