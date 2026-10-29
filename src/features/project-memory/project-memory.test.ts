import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import ProjectMemoryScreen from './ProjectMemoryScreen.svelte';
import { ProjectMemoryModel } from './project-memory-model.svelte';
import { memoryDecision } from './memory-test-fixtures';

describe('project memory editing', () => {
  it('compares a replacement with saved wording and preserves the draft on conflict', async () => {
    const save = vi.fn().mockRejectedValue(new Error('revision conflict'));
    const model = new ProjectMemoryModel({ list: vi.fn().mockResolvedValue({ items: [memoryDecision()], next_cursor: null }), save });
    render(ProjectMemoryScreen, { appId: 'app-a', capabilities: [], changes: [], model });
    await fireEvent.click(await screen.findByRole('button', { name: 'Revise decision' }));
    expect(screen.getByText('Saved revision 1')).toBeTruthy();
    await fireEvent.input(screen.getByLabelText('Decision wording'), { target: { value: 'Proposed new wording' } });
    const button = screen.getByRole('button', { name: 'Save reviewed revision' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true); await fireEvent.click(screen.getByLabelText(/I reviewed the saved and proposed wording/));
    await fireEvent.click(button); await waitFor(() => expect(save).toHaveBeenCalledWith(expect.objectContaining({ expected_revision: 1, text: 'Proposed new wording' })));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', expect.stringContaining('Your wording is still here'));
    expect(screen.getByLabelText('Decision wording')).toHaveProperty('value', 'Proposed new wording');
    expect(model.revisions[0]?.text).toBe('Decision wording 1');
  });
  it('loads imported records only on request and renders the archive as literal JSON', async () => {
    const history = vi.fn().mockResolvedValue({ evidence: '<img src=x onerror=alert(1)>', method: 'imported_untrusted', status: 'passed' });
    const model = new ProjectMemoryModel({ list: vi.fn().mockResolvedValue({ items: [], next_cursor: null }), history });
    render(ProjectMemoryScreen, { appId: 'app-a', capabilities: [], changes: [], model });
    expect(history).not.toHaveBeenCalled(); await fireEvent.click(screen.getByText('Imported archival history'));
    await fireEvent.click(screen.getByRole('button', { name: 'Load imported archive' }));
    const archive = await screen.findByLabelText('Untrusted imported archive');
    expect(archive.textContent).toContain('<img src=x onerror=alert(1)>'); expect(archive.querySelector('img')).toBeNull();
    expect(history).toHaveBeenCalledWith('app-a');
  });
});
