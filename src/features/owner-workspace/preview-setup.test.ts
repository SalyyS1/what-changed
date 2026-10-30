import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import PreviewSetup from './PreviewSetup.svelte';
import { OwnerWorkspaceStore } from '../../lib/stores/owner-workspace.svelte';
import type { OwnerEnvironmentRevisionDto, SaveOwnerEnvironmentInput } from '../../lib/tauri/bindings';
import { fixtureApp } from './owner-workspace-fixtures';
import { environmentOrigins, previewUrl } from './owner-origins';

const environment: OwnerEnvironmentRevisionDto = { id: 'environment-a', app_id: 'app-a', revision: 2, label: 'Preview', preview_url: 'https://preview.example.test/', origins: [{ scheme: 'https', host: 'preview.example.test', port: 443 }, { scheme: 'https', host: 'api.example.test', port: 443 }], kind: 'test', revoked: false, created_at: 1 };
function model() {
  const save = vi.fn(async (input: SaveOwnerEnvironmentInput) => ({ ...input, revision: input.expected_version + 1, created_at: 2 }));
  const page = async () => ({ items: [], next_cursor: null });
  return { save, store: new OwnerWorkspaceStore({ getApp: async () => fixtureApp(), listCapabilities: async () => [], listRules: page, listChanges: page, listEnvironments: async () => ({ items: [environment], next_cursor: null }), listCandidates: page, saveEnvironment: save }) };
}

describe('offline preview origin configuration', () => {
  it('retains an existing API origin when only the preview label is revised', async () => {
    const { store, save } = model(); await store.loadApp('app-a');
    render(PreviewSetup, { environments: [environment], model: store });
    expect(screen.getByText('Opt-in runner')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Edit preview' }));
    expect(screen.getByLabelText('Additional exact approved origins')).toHaveProperty('value', 'https://api.example.test:443');
    await fireEvent.input(screen.getByLabelText('Preview name'), { target: { value: 'Renamed preview' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save offline settings' }));
    await waitFor(() => expect(save).toHaveBeenCalledWith(expect.objectContaining({ expected_version: 2, label: 'Renamed preview', origins: environment.origins })));
  });
  it('authors separate exact API origins and rejects route paths before writing', async () => {
    const { store, save } = model(); await store.loadApp('app-a');
    render(PreviewSetup, { environments: [], model: store });
    await fireEvent.click(screen.getByText('Add preview settings'));
    await fireEvent.input(screen.getByLabelText('Authorized preview URL'), { target: { value: environment.preview_url } });
    await fireEvent.input(screen.getByLabelText('Additional exact approved origins'), { target: { value: 'https://api.example.test/bookings' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save offline settings' }));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', expect.stringContaining('without a route path'));
    expect(save).not.toHaveBeenCalled();
    await fireEvent.input(screen.getByLabelText('Additional exact approved origins'), { target: { value: 'https://api.example.test:443' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save offline settings' }));
    await waitFor(() => expect(save).toHaveBeenCalledWith(expect.objectContaining({ expected_version: 0, origins: environment.origins })));
  });
  it('deduplicates equivalent exact origins and bounds the complete allowlist at 16', () => {
    const preview = previewUrl(environment.preview_url);
    expect(environmentOrigins(preview, 'https://preview.example.test:443\nhttps://api.example.test\nhttps://api.example.test:443')).toEqual(environment.origins);
    expect(environmentOrigins(preview, Array.from({ length: 15 }, (_, index) => `https://api-${index}.example.test`).join('\n'))).toHaveLength(16);
    expect(() => environmentOrigins(preview, Array.from({ length: 16 }, (_, index) => `https://api-${index}.example.test`).join('\n'))).toThrow('at most 16');
  });
});
