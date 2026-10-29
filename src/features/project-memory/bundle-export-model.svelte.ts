import type { OwnerBundlePreviewDto } from '../../lib/tauri/bindings';
import { exportOwnerBundle, previewOwnerExport, toAppError } from '../../lib/tauri/client';

interface Dependencies { preview: typeof previewOwnerExport; save: typeof exportOwnerBundle; }

export class BundleExportModel {
  appId = $state<string | null>(null);
  includeTargetDetails = $state(false);
  preview = $state<OwnerBundlePreviewDto | null>(null);
  reviewed = $state(false);
  loading = $state(false);
  saving = $state(false);
  savedPath = $state<string | null>(null);
  error = $state<string | null>(null);
  #generation = 0;
  #deps: Dependencies;
  constructor(deps: Partial<Dependencies> = {}) { this.#deps = { preview: previewOwnerExport, save: exportOwnerBundle, ...deps }; }
  get canSave(): boolean { return this.appId !== null && this.preview?.app_id === this.appId && this.reviewed && !this.loading && !this.saving; }
  bind(appId: string): void { if (this.appId === appId) return; this.cancel(); this.appId = appId; }
  cancel(): void { ++this.#generation; this.appId = null; this.includeTargetDetails = false; this.clearPreview(); }
  setTargetDetails(include: boolean): void { ++this.#generation; this.includeTargetDetails = include; this.clearPreview(); }
  private clearPreview(): void { this.preview = null; this.reviewed = this.loading = this.saving = false; this.savedPath = this.error = null; }
  async prepare(): Promise<boolean> {
    const appId = this.appId; if (!appId || this.saving) return false;
    const generation = ++this.#generation; this.clearPreview(); this.loading = true;
    try {
      const preview = await this.#deps.preview(appId, { include_target_details: this.includeTargetDetails });
      if (!this.#matches(appId, generation)) return false;
      if (preview.app_id !== appId) throw new Error('The bundle preview did not match this app. Prepare a new preview.');
      this.preview = preview; return true;
    } catch (reason) { if (this.#matches(appId, generation)) this.error = toAppError(reason).message; return false; }
    finally { if (this.#matches(appId, generation)) this.loading = false; }
  }
  async save(): Promise<boolean> {
    const preview = this.preview; const appId = this.appId; if (!this.canSave || !preview || !appId) return false;
    const generation = this.#generation; this.saving = true; this.savedPath = this.error = null;
    try {
      const path = await this.#deps.save(appId, preview.digest);
      if (!this.#matches(appId, generation)) return false;
      this.savedPath = path; return path !== null;
    } catch (reason) { if (this.#matches(appId, generation)) this.error = toAppError(reason).message; return false; }
    finally { if (this.#matches(appId, generation)) this.saving = false; }
  }
  #matches(appId: string, generation: number): boolean { return this.appId === appId && this.#generation === generation; }
}
