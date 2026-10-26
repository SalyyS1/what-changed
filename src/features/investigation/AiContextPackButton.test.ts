import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import AiContextPackButton from './AiContextPackButton.svelte';
import { exportInvestigationContext } from '../../lib/tauri/client';

vi.mock('../../lib/tauri/client', () => ({
  exportInvestigationContext: vi.fn(),
}));

describe('AiContextPackButton', () => {
  beforeEach(() => {
    vi.mocked(exportInvestigationContext).mockReset();
    Object.assign(navigator, { clipboard: { writeText: vi.fn().mockResolvedValue(undefined) } });
  });

  it('copies the bounded backend pack and reports success', async () => {
    vi.mocked(exportInvestigationContext).mockResolvedValue('# evidence');
    render(AiContextPackButton, { investigationId: 7 });

    await fireEvent.click(screen.getByRole('button', { name: 'Copy context for AI' }));
    expect(exportInvestigationContext).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Copy redacted pack' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Copied for AI' })).toBeTruthy());
    expect(exportInvestigationContext).toHaveBeenCalledWith(7);
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('# evidence');
    expect(screen.getByRole('status')).toBeTruthy();
  });

  it('shows a useful error when clipboard delivery fails', async () => {
    vi.mocked(exportInvestigationContext).mockResolvedValue('# evidence');
    vi.mocked(navigator.clipboard.writeText).mockRejectedValue(new Error('Clipboard denied'));
    render(AiContextPackButton, { investigationId: 7 });

    await fireEvent.click(screen.getByRole('button', { name: 'Copy context for AI' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Copy redacted pack' }));
    expect((await screen.findByRole('alert')).textContent).toContain('Clipboard denied');
  });

  it('can cancel before the pack reaches the clipboard', async () => {
    render(AiContextPackButton, { investigationId: 7 });
    await fireEvent.click(screen.getByRole('button', { name: 'Copy context for AI' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(exportInvestigationContext).not.toHaveBeenCalled();
    expect(navigator.clipboard.writeText).not.toHaveBeenCalled();
  });
});
