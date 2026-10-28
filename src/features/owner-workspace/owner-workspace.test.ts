import { describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import OwnerWorkspaceFixture from './OwnerWorkspaceFixture.svelte';
import * as client from '../../lib/tauri/client';

describe('isolated owner workspace example', () => {
  it('shows synthetic owner context without reading production app data', async () => {
    const getApp = vi.spyOn(client, 'getOwnerApp');
    const listApps = vi.spyOn(client, 'listOwnerApps');
    const listRules = vi.spyOn(client, 'listOwnerRuleRevisions');
    const readDraft = vi.spyOn(client, 'getOwnerChangeReview');
    render(OwnerWorkspaceFixture);
    expect(await screen.findByRole('heading', { name: 'Example booking app', level: 1 })).toBeTruthy();
    expect(screen.getByTestId('synthetic-owner-workspace').textContent).toContain('no app data is saved or loaded');
    expect((screen.getByRole('button', { name: 'Request change' }) as HTMLButtonElement).disabled).toBe(true);
    expect(getApp).not.toHaveBeenCalled(); expect(listApps).not.toHaveBeenCalled();
    expect(listRules).not.toHaveBeenCalled(); expect(readDraft).not.toHaveBeenCalled();
    vi.restoreAllMocks();
  });

  it('loads the synthetic saved request through its injected store without an unsaved warning', async () => {
    const readDraft = vi.spyOn(client, 'getOwnerChangeReview');
    const listRules = vi.spyOn(client, 'listOwnerRuleRevisions');
    const saveDraft = vi.spyOn(client, 'saveOwnerChangeDraft');
    const view = render(OwnerWorkspaceFixture, { screen: 'change', state: 'ready' });
    await screen.findByDisplayValue('Add booking reminders');
    expect(screen.getByText('Saved revision 1')).toBeTruthy();
    expect(screen.queryByText('Unsaved changes')).toBeNull();
    expect(screen.getByLabelText('Desired outcome').matches(':disabled')).toBe(true);
    await view.rerender({ screen: 'change', state: 'loading' });
    await view.rerender({ screen: 'change', state: 'ready' });
    await waitFor(() => expect(screen.getByLabelText('Desired outcome')).toHaveProperty('value', 'Add booking reminders'));
    expect(screen.queryByText('Unsaved changes')).toBeNull();
    expect(readDraft).not.toHaveBeenCalled(); expect(listRules).not.toHaveBeenCalled(); expect(saveDraft).not.toHaveBeenCalled();
    vi.restoreAllMocks();
  });
});
