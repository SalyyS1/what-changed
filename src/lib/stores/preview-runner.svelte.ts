import type { PreparePreviewRunInput, PreviewRunPlanDto, PreviewRunnerStatusDto, PreviewRecipeExportDto } from '../tauri/bindings';
import { preparePreviewRun, grantPreviewRun, getPreviewRunnerStatus, continuePreviewLogin, cancelPreviewRun, exportPreviewRecipe, savePreviewRecipe, capturePreviewSurface, reviewPreviewCapture, toAppError } from '../tauri/client';
import { PreviewRevocationsStore, previewRevocations, terminalPreviewState } from './preview-revocations.svelte';

export { terminalPreviewState } from './preview-revocations.svelte';
export interface PreviewRunnerDependencies {
  prepare: typeof preparePreviewRun;
  grant: typeof grantPreviewRun;
  status: typeof getPreviewRunnerStatus;
  continueLogin: typeof continuePreviewLogin;
  cancel: typeof cancelPreviewRun;
  exportRecipe: typeof exportPreviewRecipe;
  saveRecipe: typeof savePreviewRecipe;
  capture: typeof capturePreviewSurface;
  reviewCapture: typeof reviewPreviewCapture;
}
const defaults: PreviewRunnerDependencies = { prepare: preparePreviewRun, grant: grantPreviewRun, status: getPreviewRunnerStatus, continueLogin: continuePreviewLogin, cancel: cancelPreviewRun, exportRecipe: exportPreviewRecipe, saveRecipe: savePreviewRecipe, capture: capturePreviewSurface, reviewCapture: reviewPreviewCapture };

/** Owns UI observation only. Native code owns execution, authority and verdicts. */
export class PreviewRunnerStore {
  plan = $state<PreviewRunPlanDto | null>(null);
  session = $state<PreviewRunnerStatusDto | null>(null);
  recipe = $state<PreviewRecipeExportDto | null>(null);
  busy = $state(false);
  error = $state<string | null>(null);
  saved = $state<string | null>(null);
  #generation = 0;
  #input: PreparePreviewRunInput | null = null;
  #deps: PreviewRunnerDependencies;
  readonly revocations: PreviewRevocationsStore;
  constructor(deps: Partial<PreviewRunnerDependencies> = {}, revocations?: PreviewRevocationsStore) {
    this.#deps = { ...defaults, ...deps };
    // Injected native adapters get isolated recovery state unless their caller shares an owner.
    this.revocations = revocations ?? (Object.keys(deps).length ? new PreviewRevocationsStore() : previewRevocations);
  }
  get running(): boolean { return this.session !== null && !terminalPreviewState(this.session.lifecycle); }
  revise(): void { this.plan = null; this.recipe = null; this.#input = null; this.error = null; this.saved = null; }

  async prepare(input: PreparePreviewRunInput): Promise<boolean> {
    if (this.busy || this.running) return false;
    const generation = ++this.#generation;
    this.busy = true; this.error = null; this.plan = null; this.recipe = null; this.saved = null;
    const frozen = structuredClone(input);
    try {
      const plan = await this.#deps.prepare(frozen);
      if (generation !== this.#generation) return false;
      this.plan = plan; this.#input = frozen;
      return true;
    } catch (reason) { if (generation === this.#generation) this.error = toAppError(reason).message; return false; }
    finally { if (generation === this.#generation) this.busy = false; }
  }

  async grant(): Promise<PreviewRunnerStatusDto | null> {
    if (!this.plan || this.busy || this.running) return null;
    const generation = this.#generation;
    const plan = this.plan;
    this.busy = true; this.error = null;
    try {
      const session = await this.#deps.grant({ consent_id: plan.consent_id, scope_digest: plan.scope_digest,
        authorized_test_environment: true, isolated_test_data: true, acknowledge_temporary_auth: true });
      // A grant can complete after navigation. Revoke that exact session immediately.
      if (generation !== this.#generation) { void this.revocations.revoke(session, this.#deps); return null; }
      this.session = session; this.plan = null;
      return session;
    } catch (reason) { if (generation === this.#generation) { this.error = toAppError(reason).message; this.plan = null; } return null; }
    finally { if (generation === this.#generation) this.busy = false; }
  }

  async refresh(): Promise<void> {
    const session = this.session; const generation = this.#generation;
    if (!session) return;
    try {
      const next = await this.#deps.status(session.app_id, session.run_id);
      if (generation === this.#generation && this.session?.app_id === session.app_id && this.session.run_id === session.run_id) { this.session = next; this.error = null; }
    } catch (reason) {
      if (generation === this.#generation) this.error = `${toAppError(reason).message} Execution status is unknown. Revoke permission or retry reading status.`;
    }
  }

  async continueLogin(): Promise<void> {
    const session = this.session; const generation = this.#generation;
    if (!session?.can_continue || this.busy) return;
    this.busy = true;
    try { await this.#deps.continueLogin(session.app_id, session.run_id); await this.refresh(); }
    catch (reason) { if (generation === this.#generation) this.error = toAppError(reason).message; }
    finally { if (generation === this.#generation) this.busy = false; }
  }

  async cancel(): Promise<void> {
    const session = this.session; const generation = this.#generation;
    if (!session || !this.running) return;
    const next = await this.revocations.revoke(session, this.#deps);
    if (generation !== this.#generation || this.session?.app_id !== session.app_id || this.session.run_id !== session.run_id) return;
    if (next) this.session = next;
    this.error = next && terminalPreviewState(next.lifecycle) ? null : 'Permission revocation is not confirmed. Retry using the preview revocation controls.';
  }

  async previewExport(): Promise<void> {
    if (!this.#input || this.busy || this.running) return;
    const generation = this.#generation;
    this.busy = true; this.error = null;
    try { const recipe = await this.#deps.exportRecipe(this.#input); if (generation === this.#generation) this.recipe = recipe; }
    catch (reason) { if (generation === this.#generation) this.error = toAppError(reason).message; }
    finally { if (generation === this.#generation) this.busy = false; }
  }

  async saveExport(digest: string): Promise<void> {
    if (!this.#input || !this.recipe || this.busy || this.running) return;
    const generation = this.#generation;
    this.busy = true;
    try { const saved = await this.#deps.saveRecipe(this.#input, digest); if (generation === this.#generation) this.saved = saved; }
    catch (reason) { if (generation === this.#generation) this.error = toAppError(reason).message; }
    finally { if (generation === this.#generation) this.busy = false; }
  }

  async capture(role: string): Promise<void> {
    const session = this.session;
    if (!session || !this.running || !session.capture_screenshots || this.busy) return;
    await this.#captureAction(() => this.#deps.capture(session.app_id, session.run_id, role));
  }
  async reviewCapture(captureId: string, retain: boolean): Promise<void> {
    const session = this.session;
    if (!session || this.busy) return;
    await this.#captureAction(() => this.#deps.reviewCapture(session.app_id, session.run_id, captureId, retain));
  }
  async #captureAction(action: () => Promise<void>): Promise<void> {
    const generation = this.#generation; this.busy = true; this.error = null;
    try { await action(); await this.refresh(); }
    catch (reason) { if (generation === this.#generation) this.error = toAppError(reason).message; }
    finally { if (generation === this.#generation) this.busy = false; }
  }

  /** Navigation revokes an owned active session; it never launches one. */
  dispose(): void {
    ++this.#generation;
    const session = this.session;
    if (session && !terminalPreviewState(session.lifecycle)) void this.revocations.revoke(session, this.#deps);
    this.session = null; this.plan = null; this.recipe = null; this.busy = false; this.#input = null;
  }
}
