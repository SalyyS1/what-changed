import { describe, expect, it, vi } from 'vitest';
import type { OwnerBriefPreviewDto } from '../tauri/bindings';
import { OwnerBriefStore } from './owner-brief.svelte';
import { deferred } from '../../features/owner-workspace/owner-workspace-fixtures';

const preview: OwnerBriefPreviewDto = {
  app_id: 'app-a', change_id: 'change-a', change_version: 1,
  markdown: '# Synthetic brief\n\nNo run was started.', bytes: 44, estimated_tokens: 11,
  omitted_sections: ['environment_details'], limitations: ['No candidate selected.'],
};

describe('OwnerBriefStore', () => {
  it('copies only the reviewed preview after an explicit action', async () => {
    const copy = vi.fn().mockResolvedValue(undefined);
    const store = new OwnerBriefStore({ preview: vi.fn().mockResolvedValue(preview), copy });
    store.bind('app-a', 'change-a', 1);
    expect(await store.copy()).toBe(false);
    await store.load();
    expect(await store.copy()).toBe(false);
    store.reviewed = true;
    expect(copy).not.toHaveBeenCalled();
    expect(await store.copy()).toBe(true);
    expect(copy).toHaveBeenCalledWith(preview.markdown);
    expect(store.copied).toBe(true);
  });

  it('keeps a reviewed preview available when clipboard writing fails', async () => {
    const store = new OwnerBriefStore({ preview: vi.fn().mockResolvedValue(preview), copy: vi.fn().mockRejectedValue(new Error('denied')) });
    store.bind('app-a', 'change-a', 1); await store.load(); store.reviewed = true;
    expect(await store.copy()).toBe(false);
    expect(store.preview?.markdown).toBe(preview.markdown);
    expect(store.copied).toBe(false); expect(store.copying).toBe(false);
    expect(store.error).toContain('could not be copied');
  });

  it('discards a late preview after changing app or request version', async () => {
    const pending = deferred<OwnerBriefPreviewDto>();
    const store = new OwnerBriefStore({ preview: vi.fn(() => pending.promise) });
    store.bind('app-a', 'change-a', 1); const loading = store.load();
    store.bind('app-b', 'change-b', 2); pending.resolve(preview);
    expect(await loading).toBe(false); expect(store.preview).toBeNull(); expect(store.canCopy).toBe(false);
    expect(store.appId).toBe('app-b'); expect(store.version).toBe(2);
  });

  it('invalidates preview and review when section selection changes', async () => {
    const read = vi.fn().mockResolvedValue(preview);
    const store = new OwnerBriefStore({ preview: read });
    store.bind('app-a', 'change-a', 1); await store.load(); store.reviewed = true;
    store.setSelection({ ...store.selection, include_environment_details: true });
    expect(store.preview).toBeNull(); expect(store.reviewed).toBe(false); expect(store.canCopy).toBe(false);
    await store.load(); expect(read).toHaveBeenLastCalledWith('app-a', 'change-a', expect.objectContaining({ include_environment_details: true }));
  });

  it('discards late clipboard feedback after a different request is opened', async () => {
    const pending = deferred<void>();
    const store = new OwnerBriefStore({ preview: vi.fn().mockResolvedValue(preview), copy: vi.fn(() => pending.promise) });
    store.bind('app-a', 'change-a', 1); await store.load(); store.reviewed = true;
    const copying = store.copy(); store.bind('app-b', 'change-b', 1); pending.resolve();
    expect(await copying).toBe(false); expect(store.copied).toBe(false); expect(store.preview).toBeNull();
  });

  it('blocks copying a preview rendered from a different saved request version', async () => {
    const store = new OwnerBriefStore({ preview: vi.fn().mockResolvedValue({ ...preview, change_version: 2 }) });
    store.bind('app-a', 'change-a', 1);
    expect(await store.load()).toBe(false);
    expect(store.preview).toBeNull(); expect(store.canCopy).toBe(false);
    expect(store.error).toContain('request changed');
  });
});
