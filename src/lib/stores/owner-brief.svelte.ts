import type { OwnerBriefPreviewDto, OwnerBriefSelectionInput } from '../tauri/bindings';
import { previewOwnerBrief, toAppError } from '../tauri/client';

export interface OwnerBriefDependencies {
  preview: typeof previewOwnerBrief;
  copy: (text: string) => Promise<void>;
}
const defaults: OwnerBriefDependencies = {
  preview: previewOwnerBrief,
  copy: (text) => navigator.clipboard.writeText(text),
};

export function defaultBriefSelection(): OwnerBriefSelectionInput {
  return { include_scenarios: true, scenario_ids: [], include_candidate_identity: false, include_environment_details: false, include_limitations: true };
}

/** A preview belongs to one saved request version and one explicit section selection. */
export class OwnerBriefStore {
  appId = $state<string | null>(null);
  changeId = $state<string | null>(null);
  version = $state<number | null>(null);
  selection = $state<OwnerBriefSelectionInput>(defaultBriefSelection());
  preview = $state<OwnerBriefPreviewDto | null>(null);
  loading = $state(false);
  copying = $state(false);
  reviewed = $state(false);
  copied = $state(false);
  error = $state<string | null>(null);
  #generation = 0;
  #deps: OwnerBriefDependencies;

  constructor(deps: Partial<OwnerBriefDependencies> = {}) { this.#deps = { ...defaults, ...deps }; }
  get canCopy(): boolean { return this.preview !== null && this.reviewed && !this.loading && !this.copying && this.appId !== null && this.changeId !== null && this.version !== null; }

  bind(appId: string, changeId: string, version: number): void {
    if (this.appId === appId && this.changeId === changeId && this.version === version) return;
    this.cancel();
    this.appId = appId; this.changeId = changeId; this.version = version;
    this.selection = defaultBriefSelection();
  }

  cancel(): void {
    ++this.#generation;
    this.appId = this.changeId = null; this.version = null;
    this.preview = null; this.loading = this.copying = this.reviewed = this.copied = false;
    this.error = null;
  }

  setSelection(selection: OwnerBriefSelectionInput): void {
    ++this.#generation;
    this.selection = { ...selection, scenario_ids: selection.scenario_ids.map((ref) => ({ ...ref })) };
    this.preview = null; this.loading = this.copying = this.reviewed = this.copied = false; this.error = null;
  }

  async load(): Promise<boolean> {
    const appId = this.appId; const changeId = this.changeId; const version = this.version;
    if (!appId || !changeId || version === null) return false;
    const generation = ++this.#generation;
    this.loading = true; this.preview = null; this.reviewed = this.copied = false; this.error = null;
    try {
      const selection = { ...this.selection, scenario_ids: this.selection.scenario_ids.map((ref) => ({ ...ref })) };
      const preview = await this.#deps.preview(appId, changeId, selection);
      if (!this.#matches(appId, changeId, version, generation)) return false;
      if (preview.app_id !== appId || preview.change_id !== changeId || preview.change_version !== version) {
        this.error = 'The saved request changed while preparing this preview. Close this review, reopen the saved request, and review its latest version before copying.';
        return false;
      }
      this.preview = preview;
      return true;
    } catch (reason) {
      if (this.#matches(appId, changeId, version, generation)) this.error = toAppError(reason).message;
      return false;
    } finally {
      if (this.#matches(appId, changeId, version, generation)) this.loading = false;
    }
  }

  async copy(): Promise<boolean> {
    const preview = this.preview;
    if (!this.canCopy || !preview) return false;
    const generation = this.#generation; const appId = this.appId!; const changeId = this.changeId!; const version = this.version!;
    this.copying = true; this.copied = false; this.error = null;
    try {
      await this.#deps.copy(preview.markdown);
      if (!this.#matches(appId, changeId, version, generation)) return false;
      this.copied = true;
      return true;
    } catch {
      if (this.#matches(appId, changeId, version, generation)) this.error = 'The brief could not be copied. Keep this review open and try again.';
      return false;
    } finally {
      if (this.#matches(appId, changeId, version, generation)) this.copying = false;
    }
  }

  #matches(appId: string, changeId: string, version: number, generation: number): boolean {
    return generation === this.#generation && appId === this.appId && changeId === this.changeId && version === this.version;
  }
}
