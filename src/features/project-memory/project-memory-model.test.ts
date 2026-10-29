import { describe, expect, it, vi } from 'vitest';
import type { OwnerBundlePreviewDto, OwnerImportPreviewDto, OwnerPageDto, ProjectDecisionRevisionDto } from '../../lib/tauri/bindings';
import { ProjectMemoryModel, latestDecisions } from './project-memory-model.svelte';
import { BundleExportModel } from './bundle-export-model.svelte';
import { BundleImportModel, ruleKey } from './bundle-import-model.svelte';
import { bundlePreview, deferred, importPreview, memoryApp, memoryDecision } from './memory-test-fixtures';

describe('project memory revision boundaries', () => {
  it('loads all immutable revisions and saves against the exact prior revision', async () => {
    const list = vi.fn().mockResolvedValueOnce({ items: [memoryDecision()], next_cursor: 'next' }).mockResolvedValueOnce({ items: [memoryDecision(2)], next_cursor: null });
    const save = vi.fn().mockResolvedValue(memoryDecision(3));
    const model = new ProjectMemoryModel({ list, save }); model.bind('app-a');
    await vi.waitFor(() => expect(model.revisions).toHaveLength(2));
    expect(list.mock.calls[1]?.[1].page.after).toBe('next');
    const prior = latestDecisions(model.revisions)[0]!;
    expect(await model.save({ text: 'New wording', rationale: 'New reason', recorded_by: 'Owner', provenance: 'owner', capability_id: null, change_id: null }, prior)).toBe(true);
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ id: prior.id, expected_revision: 2 }));
    expect(model.revisions.map((item) => item.revision)).toEqual([1, 2, 3]);
    expect(model.revisions[0]?.text).toBe('Decision wording 1');
  });
  it('ignores another app history and keeps a rejected revision intact', async () => {
    const old = deferred<OwnerPageDto<ProjectDecisionRevisionDto>>();
    const list = vi.fn().mockImplementation((appId: string) => appId === 'app-a' ? old.promise : Promise.resolve({ items: [memoryDecision(1, 'app-b')], next_cursor: null }));
    const save = vi.fn().mockRejectedValue(new Error('revision conflict'));
    const model = new ProjectMemoryModel({ list, save }); model.bind('app-a'); model.bind('app-b');
    await vi.waitFor(() => expect(model.loading).toBe(false)); old.resolve({ items: [memoryDecision()], next_cursor: null });
    await old.promise;
    expect(model.revisions.every((item) => item.app_id === 'app-b')).toBe(true);
    expect(await model.save({ text: 'Preserve this draft', rationale: 'Reason', recorded_by: 'Owner', provenance: 'owner', capability_id: null, change_id: null }, model.revisions[0])).toBe(false);
    expect(model.revisions).toHaveLength(1); expect(model.error).toContain('Your wording is still here');
  });
});

describe('reviewed bundle export', () => {
  it('saves only the reviewed digest and invalidates review when target inclusion changes', async () => {
    const preview = vi.fn().mockResolvedValue(bundlePreview()); const save = vi.fn().mockResolvedValue('C:/reviewed.zip');
    const model = new BundleExportModel({ preview, save }); model.bind('app-a'); await model.prepare();
    expect(await model.save()).toBe(false); expect(save).not.toHaveBeenCalled();
    model.reviewed = true; expect(await model.save()).toBe(true); expect(save).toHaveBeenCalledWith('app-a', 'digest-a');
    model.setTargetDetails(true); expect(model.preview).toBeNull(); expect(model.reviewed).toBe(false);
    await model.prepare(); expect(preview).toHaveBeenLastCalledWith('app-a', { include_target_details: true });
  });
  it('discards a late preview from an earlier selection or app', async () => {
    const pending = deferred<OwnerBundlePreviewDto>(); const model = new BundleExportModel({ preview: vi.fn().mockReturnValue(pending.promise) });
    model.bind('app-a'); const load = model.prepare(); model.setTargetDetails(true); pending.resolve(bundlePreview());
    expect(await load).toBe(false); expect(model.preview).toBeNull(); expect(model.loading).toBe(false);
    model.bind('app-b'); expect(model.preview).toBeNull(); expect(model.includeTargetDetails).toBe(false);
  });
});

describe('reviewed bundle import', () => {
  it('requires every historical rule, author and review before applying exact validated identity', async () => {
    const preview = importPreview(); const apply = vi.fn().mockResolvedValue({ app: memoryApp, source_digest: preview.digest, limitations: [] });
    const model = new BundleImportModel({ validate: vi.fn().mockResolvedValue(preview), apply }); await model.choose();
    model.confirmedBy = 'New owner'; model.reviewed = true;
    model.confirmRule(ruleKey('rule-a', 2), true); expect(model.canApply).toBe(false);
    expect(await model.apply()).toBeNull(); expect(apply).not.toHaveBeenCalled();
    model.confirmRule(ruleKey('rule-a', 1), true); expect(model.canApply).toBe(true);
    expect(await model.apply()).toEqual(expect.objectContaining({ app: memoryApp }));
    expect(apply).toHaveBeenCalledWith(expect.objectContaining({ review_id: preview.review_id, digest: preview.digest, confirmed_by: 'New owner', confirmed_rules: [{ rule_id: 'rule-a', revision: 1 }, { rule_id: 'rule-a', revision: 2 }] }));
    expect(model.canApply).toBe(false);
  });
  it('performs no write during validation and clears a cancelled or stale review', async () => {
    const pending = deferred<OwnerImportPreviewDto | null>(); const apply = vi.fn(); const model = new BundleImportModel({ validate: vi.fn().mockReturnValue(pending.promise), apply });
    const choice = model.choose(); model.cancel(); pending.resolve(importPreview()); expect(await choice).toBe(false);
    expect(model.preview).toBeNull(); expect(apply).not.toHaveBeenCalled();
    const cancelled = new BundleImportModel({ validate: vi.fn().mockResolvedValue(null), apply });
    expect(await cancelled.choose()).toBe(false); expect(cancelled.preview).toBeNull(); expect(cancelled.error).toBeNull();
  });
});
