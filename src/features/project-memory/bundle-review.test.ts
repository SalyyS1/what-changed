import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import BundleExportReview from './BundleExportReview.svelte';
import ImportReview from './ImportReview.svelte';
import { BundleExportModel } from './bundle-export-model.svelte';
import { BundleImportModel } from './bundle-import-model.svelte';
import { bundlePreview, importPreview, memoryApp } from './memory-test-fixtures';

describe('bundle review surfaces', () => {
  it('shows literal Markdown and requires explicit review before native file export', async () => {
    const save = vi.fn().mockResolvedValue('C:/reviewed.zip'); const model = new BundleExportModel({ preview: vi.fn().mockResolvedValue(bundlePreview()), save });
    render(BundleExportReview, { appId: 'app-a', model });
    await fireEvent.click(screen.getByRole('button', { name: 'Prepare bundle preview' }));
    expect(await screen.findByLabelText('Exact bundle Markdown')).toHaveProperty('textContent', '<script>literal bundle text</script>');
    const button = screen.getByRole('button', { name: 'Save reviewed bundle' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true); await fireEvent.click(screen.getByLabelText(/I reviewed this bundle/));
    await fireEvent.click(button); await waitFor(() => expect(save).toHaveBeenCalledWith('app-a', 'digest-a'));
    await fireEvent.click(screen.getByLabelText(/Include selected preview target details/));
    expect(screen.queryByLabelText('Exact bundle Markdown')).toBeNull(); expect(model.reviewed).toBe(false);
  });
  it('requires an individual confirmation for each old and new imported rule', async () => {
    const apply = vi.fn().mockResolvedValue({ app: memoryApp, source_digest: 'digest-a', limitations: [] }); const onImported = vi.fn();
    const model = new BundleImportModel({ validate: vi.fn().mockResolvedValue(importPreview()), apply });
    render(ImportReview, { model, onImported }); await fireEvent.click(screen.getByRole('button', { name: 'Choose bundle to review' }));
    expect(await screen.findByLabelText('Imported bundle Markdown')).toHaveProperty('textContent', '<script>literal import text</script>');
    await fireEvent.input(screen.getByLabelText('Confirmed by'), { target: { value: 'Reviewer' } });
    await fireEvent.click(screen.getByLabelText(/I reviewed the new app identity/));
    await fireEvent.click(screen.getByLabelText(/Confirm rule revision 2/));
    const button = screen.getByRole('button', { name: 'Import reviewed bundle' }) as HTMLButtonElement;
    expect(button.disabled).toBe(true); await fireEvent.click(screen.getByLabelText(/Confirm rule revision 1/)); expect(button.disabled).toBe(false);
    await fireEvent.click(button); await waitFor(() => expect(onImported).toHaveBeenCalledTimes(1));
    expect(await screen.findByText(/Imported Imported booking app as a separate app/)).toBeTruthy();
  });
  it('keeps inactive synthetic review surfaces disconnected from native commands', () => {
    const preview = vi.fn(); const validate = vi.fn();
    render(BundleExportReview, { appId: 'synthetic', active: false, model: new BundleExportModel({ preview }) });
    render(ImportReview, { active: false, model: new BundleImportModel({ validate }) });
    expect((screen.getByRole('button', { name: 'Prepare bundle preview' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'Choose bundle to review' }) as HTMLButtonElement).disabled).toBe(true);
    expect(preview).not.toHaveBeenCalled(); expect(validate).not.toHaveBeenCalled();
  });
});
