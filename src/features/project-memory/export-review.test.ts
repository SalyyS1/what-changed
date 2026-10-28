import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import ExportReview from './ExportReview.svelte';
import { OwnerBriefStore } from '../../lib/stores/owner-brief.svelte';

describe('ExportReview', () => {
  it('renders exact text safely and requires owner review before copying', async () => {
    const copy = vi.fn().mockResolvedValue(undefined);
    const model = new OwnerBriefStore({ preview: vi.fn().mockResolvedValue({ app_id: 'app-a', change_id: 'change-a', change_version: 1, markdown: '<script>not executed</script>', bytes: 29, estimated_tokens: 8, omitted_sections: ['private preview origins'], limitations: ['No test run.'] }), copy });
    render(ExportReview, { appId: 'app-a', changeId: 'change-a', version: 1, model });
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare preview' }));
    expect(await screen.findByLabelText('Exact brief text')).toHaveProperty('textContent', '<script>not executed</script>');
    const button = screen.getByRole('button', { name: 'Copy reviewed brief' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    await fireEvent.click(screen.getByLabelText(/I reviewed this text/));
    expect(button.disabled).toBe(false);
    await fireEvent.click(button);
    await waitFor(() => expect(copy).toHaveBeenCalledWith('<script>not executed</script>'));
    expect(await screen.findByText('Copied. Paste it into your coding AI when ready.')).toBeTruthy();
  });

  it('clears and disables preview when the request becomes dirty', async () => {
    const model = new OwnerBriefStore({ preview: vi.fn().mockResolvedValue({ app_id: 'app-a', change_id: 'change-a', change_version: 1, markdown: 'Saved version', bytes: 13, estimated_tokens: 4, omitted_sections: [], limitations: [] }) });
    const view = render(ExportReview, { appId: 'app-a', changeId: 'change-a', version: 1, model });
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare preview' }));
    await screen.findByLabelText('Exact brief text');
    await view.rerender({ appId: 'app-a', changeId: 'change-a', version: 1, model, dirty: true });
    await waitFor(() => expect(screen.queryByLabelText('Exact brief text')).toBeNull());
    expect((screen.getByRole('button', { name: 'Prepare preview' }) as HTMLButtonElement).disabled).toBe(true);
    expect(model.preview).toBeNull();
  });
});
