import type {
  OwnerChangeReviewDto, OwnerEvidenceDecisionDto, OwnerEvidenceReviewDto, OwnerManualRunDto,
  OwnerCandidateRevisionDto, ScenarioRevisionDto, ConfirmOwnerScenarioInput, ScenarioRefDto,
  RecordOwnerManualResultInput, RecordOwnerEvidenceDecisionInput, RunLifecycle,
} from '../tauri/bindings';
import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import {
  getOwnerChangeReview, listOwnerScenarios, confirmOwnerScenario, listOwnerCandidates, saveOwnerChangeDraft,
  startOwnerManualRun, recordOwnerManualResult, finishOwnerManualRun, getOwnerManualRun,
  listOwnerManualRuns, recordOwnerEvidenceDecision, listOwnerEvidenceDecisions, toAppError,
} from '../tauri/client';
import { ownerPages } from './owner-workspace.svelte';

export interface OwnerEvidenceContext {
  change: OwnerChangeReviewDto;
  scenarios: ScenarioRevisionDto[];
  candidates: OwnerCandidateRevisionDto[];
  runs: OwnerManualRunDto[];
  decisions: OwnerEvidenceDecisionDto[];
}
export interface OwnerEvidenceDependencies {
  getChange: typeof getOwnerChangeReview;
  listScenarios: typeof listOwnerScenarios;
  confirmScenario: typeof confirmOwnerScenario;
  listCandidates: typeof listOwnerCandidates;
  saveChange: typeof saveOwnerChangeDraft;
  start: typeof startOwnerManualRun;
  record: typeof recordOwnerManualResult;
  finish: typeof finishOwnerManualRun;
  getRun: typeof getOwnerManualRun;
  listRuns: typeof listOwnerManualRuns;
  decide: typeof recordOwnerEvidenceDecision;
  listDecisions: typeof listOwnerEvidenceDecisions;
  uuid: () => string;
}
const defaults: OwnerEvidenceDependencies = {
  getChange: getOwnerChangeReview, listScenarios: listOwnerScenarios, confirmScenario: confirmOwnerScenario,
  listCandidates: listOwnerCandidates, saveChange: saveOwnerChangeDraft,
  start: startOwnerManualRun, record: recordOwnerManualResult, finish: finishOwnerManualRun,
  getRun: getOwnerManualRun, listRuns: listOwnerManualRuns, decide: recordOwnerEvidenceDecision,
  listDecisions: listOwnerEvidenceDecisions, uuid: () => crypto.randomUUID(),
};
export type OwnerScenarioFields = Omit<ConfirmOwnerScenarioInput, 'operation_id' | 'id' | 'app_id' | 'expected_version'>;
export type OwnerObservationFields = Pick<RecordOwnerManualResultInput, 'scenario_id' | 'scenario_revision' | 'observer' | 'observed_at' | 'observed_steps' | 'assertions' | 'limitation'>;
export type OwnerDecisionFields = Pick<RecordOwnerEvidenceDecisionInput, 'decision' | 'reason' | 'acknowledged_unresolved' | 'acknowledge_unknown_build'>;

/** Backend review owns status, freshness, gaps and decision eligibility. */
export class OwnerEvidenceStore {
  state = $state<AsyncState<OwnerEvidenceContext>>(loading());
  review = $state<AsyncState<OwnerEvidenceReviewDto>>(empty());
  appId = $state<string | null>(null);
  changeId = $state<string | null>(null);
  runId = $state<string | null>(null);
  saving = $state(false);
  error = $state<string | null>(null);
  #generation = 0;
  #runGeneration = 0;
  #deps: OwnerEvidenceDependencies;
  constructor(deps: Partial<OwnerEvidenceDependencies> = {}) { this.#deps = { ...defaults, ...deps }; }
  get context(): OwnerEvidenceContext | null { return this.state.status === 'ready' ? this.state.data : null; }
  get currentReview(): OwnerEvidenceReviewDto | null { return this.review.status === 'ready' ? this.review.data : null; }
  get latestScenarios(): ScenarioRevisionDto[] {
    const rows = this.context?.scenarios ?? [];
    return rows.filter((item) => !rows.some((other) => item.id === other.id && other.revision > item.revision));
  }
  cancel(): void {
    ++this.#generation; ++this.#runGeneration;
    this.appId = this.changeId = this.runId = null;
    this.state = loading(); this.review = empty(); this.saving = false; this.error = null;
  }
  async load(appId: string, changeId: string): Promise<void> {
    this.cancel(); this.appId = appId; this.changeId = changeId;
    await this.#refresh(appId, changeId, this.#generation);
  }
  async reload(runId: string | null = this.runId): Promise<void> {
    const appId = this.appId; const changeId = this.changeId; const generation = this.#generation;
    if (!appId || !changeId) return;
    await this.#refresh(appId, changeId, generation);
    if (runId && this.#matches(appId, generation) && this.changeId === changeId) await this.selectRun(runId);
  }
  async selectRun(runId: string): Promise<void> {
    const appId = this.appId; const generation = this.#generation;
    if (!appId) return;
    const request = ++this.#runGeneration;
    this.runId = runId; this.review = loading();
    try {
      const review = await this.#deps.getRun(appId, runId);
      if (this.#matches(appId, generation) && request === this.#runGeneration && this.runId === runId) this.review = ready(review);
    } catch (reason) {
      if (this.#matches(appId, generation) && request === this.#runGeneration) this.review = failed(toAppError(reason).message, () => void this.selectRun(runId));
    }
  }
  confirmScenario(fields: OwnerScenarioFields, previous?: ScenarioRevisionDto): Promise<boolean> {
    return this.#write(async (appId) => {
      await this.#deps.confirmScenario({ ...fields, operation_id: this.#deps.uuid(), id: previous?.id ?? this.#deps.uuid(), app_id: appId, expected_version: previous?.revision ?? 0 });
      return this.runId;
    });
  }
  bindSelectedCandidate(): Promise<boolean> {
    if (!this.context?.change.app.current_candidate_id) return Promise.resolve(false);
    return this.#write(async (appId) => {
      const review = this.context!.change;
      if (!review.app.current_candidate_id) return null;
      const change = review.change;
      await this.#deps.saveChange({ operation_id: this.#deps.uuid(), id: change.id, app_id: appId,
        expected_version: change.version, intent: change.intent, example: change.example, scope: change.scope,
        preserved_rules: change.preserved_rules, candidate_id: review.app.current_candidate_id, state: change.state });
      return this.runId;
    });
  }
  start(scenarios: ScenarioRefDto[], retryOf: string | null = null): Promise<boolean> {
    return this.#write(async (appId) => {
      const change = this.context!.change.change;
      if (!change.candidate_id || !scenarios.length) throw { code: 'validation', message: 'Save a candidate for this request and choose confirmed scenarios before starting an observation.', retryable: false };
      const run = await this.#deps.start({ operation_id: this.#deps.uuid(), id: this.#deps.uuid(), app_id: appId,
        change_id: change.id, expected_change_version: change.version, candidate_id: change.candidate_id,
        scenarios: scenarios.map((ref) => ({ ...ref })), retry_of: retryOf });
      return run.id;
    });
  }
  record(fields: OwnerObservationFields): Promise<boolean> {
    if (!this.currentReview) return Promise.resolve(false);
    return this.#write(async (appId) => {
      const run = this.currentReview?.run;
      if (!run) return null;
      await this.#deps.record({ ...fields, operation_id: this.#deps.uuid(), id: this.#deps.uuid(), app_id: appId, run_id: run.id, expected_run_version: run.version });
      return run.id;
    });
  }
  finish(lifecycle: RunLifecycle): Promise<boolean> {
    if (!this.currentReview) return Promise.resolve(false);
    return this.#write(async (appId) => {
      const run = this.currentReview?.run;
      if (!run) return null;
      await this.#deps.finish({ operation_id: this.#deps.uuid(), app_id: appId, run_id: run.id, expected_run_version: run.version, lifecycle });
      return run.id;
    });
  }
  decide(fields: OwnerDecisionFields): Promise<boolean> {
    if (!this.currentReview) return Promise.resolve(false);
    return this.#write(async (appId) => {
      const run = this.currentReview?.run;
      if (!run || !this.context) return null;
      await this.#deps.decide({ ...fields, operation_id: this.#deps.uuid(), id: this.#deps.uuid(), app_id: appId,
        change_id: run.change_id, expected_change_version: this.context.change.change.version,
        run_id: run.id, expected_run_version: run.version });
      return run.id;
    });
  }
  #matches(appId: string, generation: number): boolean { return this.appId === appId && generation === this.#generation; }
  async #refresh(appId: string, changeId: string, generation: number): Promise<void> {
    try {
      const [change, scenarios, candidates, runReviews, decisions] = await Promise.all([
        this.#deps.getChange(appId, changeId), ownerPages((page) => this.#deps.listScenarios(appId, page)),
        ownerPages((page) => this.#deps.listCandidates(appId, page)), ownerPages((page) => this.#deps.listRuns(appId, changeId, page)),
        ownerPages((page) => this.#deps.listDecisions(appId, changeId, page)),
      ]);
      if (this.#matches(appId, generation)) this.state = ready({ change, scenarios, candidates, runs: runReviews.map((review) => review.run), decisions });
    } catch (reason) {
      if (this.#matches(appId, generation)) this.state = failed(toAppError(reason).message, () => void this.load(appId, changeId));
    }
  }
  async #write(write: (appId: string) => Promise<string | null>): Promise<boolean> {
    const appId = this.appId; const changeId = this.changeId; const generation = this.#generation;
    const selectedRunGeneration = this.#runGeneration;
    if (!appId || !changeId || !this.context || this.saving || this.context.change.app.archived_at !== null) return false;
    this.saving = true; this.error = null;
    try {
      const runId = await write(appId);
      if (!this.#matches(appId, generation) || this.changeId !== changeId) return false;
      await this.#refresh(appId, changeId, generation);
      if (!this.#matches(appId, generation)) return false;
      if (runId && selectedRunGeneration === this.#runGeneration) await this.selectRun(runId);
      return this.state.status === 'ready';
    } catch (reason) {
      if (this.#matches(appId, generation)) this.error = toAppError(reason).message;
      return false;
    } finally { if (this.#matches(appId, generation)) this.saving = false; }
  }
}
