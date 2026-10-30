import type { OwnerImportPreviewDto, OwnerImportResultDto } from '../../lib/tauri/bindings';
import { importOwnerBundle, toAppError, validateOwnerImport } from '../../lib/tauri/client';

interface Dependencies { validate: typeof validateOwnerImport; apply: typeof importOwnerBundle; }
export function ruleKey(ruleId: string, revision: number): string { return `${ruleId}:${revision}`; }

export class BundleImportModel {
  preview = $state<OwnerImportPreviewDto | null>(null);
  name = $state('');
  confirmedBy = $state('');
  confirmedRules = $state<string[]>([]);
  reviewed = $state(false);
  loading = $state(false);
  applying = $state(false);
  result = $state<OwnerImportResultDto | null>(null);
  error = $state<string | null>(null);
  #generation = 0;
  #deps: Dependencies;
  constructor(deps: Partial<Dependencies> = {}) { this.#deps = { validate: validateOwnerImport, apply: importOwnerBundle, ...deps }; }
  get canApply(): boolean {
    return this.preview !== null && this.name.trim().length > 0 && this.confirmedBy.trim().length > 0 && this.reviewed && !this.loading && !this.applying && this.result === null
      && this.preview.rules.every((rule) => this.confirmedRules.includes(ruleKey(rule.rule_id, rule.revision)));
  }
  cancel(): void { ++this.#generation; this.preview = this.result = null; this.name = this.confirmedBy = ''; this.confirmedRules = []; this.reviewed = this.loading = this.applying = false; this.error = null; }
  confirmRule(key: string, confirmed: boolean): void { this.confirmedRules = confirmed ? [...new Set([...this.confirmedRules, key])] : this.confirmedRules.filter((item) => item !== key); }
  async choose(): Promise<boolean> {
    if (this.applying) return false; this.cancel(); const generation = this.#generation; this.loading = true;
    try {
      const preview = await this.#deps.validate();
      if (generation !== this.#generation || preview === null) return false;
      this.preview = preview; this.name = preview.app_name; return true;
    } catch (reason) { if (generation === this.#generation) this.error = toAppError(reason).message; return false; }
    finally { if (generation === this.#generation) this.loading = false; }
  }
  async apply(): Promise<OwnerImportResultDto | null> {
    const preview = this.preview; if (!this.canApply || !preview) return null;
    const generation = this.#generation; this.applying = true; this.error = null;
    try {
      const result = await this.#deps.apply({ operation_id: crypto.randomUUID(), review_id: preview.review_id, digest: preview.digest, name: this.name.trim(), confirmed_by: this.confirmedBy.trim(), confirmed_rules: preview.rules.map((rule) => ({ rule_id: rule.rule_id, revision: rule.revision })) });
      if (generation !== this.#generation) return null;
      if (result.app.id !== preview.proposed_app_id || result.source_digest !== preview.digest) throw new Error('The imported app did not match this review. Reload My apps before continuing.');
      this.result = result; return result;
    } catch (reason) { if (generation === this.#generation) this.error = toAppError(reason).message; return null; }
    finally { if (generation === this.#generation) this.applying = false; }
  }
}
