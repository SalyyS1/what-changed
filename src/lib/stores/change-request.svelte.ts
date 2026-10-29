import { failed, loading, ready, type AsyncState } from '../types/async-state';
import type {
  OwnerAppDto, OwnerRuleRevisionDto, OwnerChangeDraftDto, RuleRefDto, ChangeState,
} from '../tauri/bindings';
import { getOwnerApp, getOwnerChangeReview, listOwnerRuleRevisions, saveOwnerChangeDraft, toAppError } from '../tauri/client';
import { latestOwnerRules, ownerPages } from './owner-workspace.svelte';

export interface ChangeRequestContext { app: OwnerAppDto; rules: OwnerRuleRevisionDto[]; }
export interface ChangeRequestDependencies {
  getApp: typeof getOwnerApp;
  getReview: typeof getOwnerChangeReview;
  listRules: typeof listOwnerRuleRevisions;
  save: typeof saveOwnerChangeDraft;
  uuid: () => string;
}
const defaults: ChangeRequestDependencies = {
  getApp: getOwnerApp, getReview: getOwnerChangeReview, listRules: listOwnerRuleRevisions,
  save: saveOwnerChangeDraft, uuid: () => crypto.randomUUID(),
};

export class ChangeRequestStore {
  state = $state<AsyncState<ChangeRequestContext>>(loading());
  appId = $state<string | null>(null);
  changeId = $state<string | null>(null);
  draft = $state<OwnerChangeDraftDto | null>(null);
  intent = $state('');
  example = $state('');
  scope = $state('');
  preservedRules = $state<RuleRefDto[]>([]);
  saving = $state(false);
  error = $state<string | null>(null);
  conflict = $state(false);
  saved = $state(false);
  #baseline = $state('');
  #generation = 0;
  #deps: ChangeRequestDependencies;

  constructor(deps: Partial<ChangeRequestDependencies> = {}) { this.#deps = { ...defaults, ...deps }; }
  get context(): ChangeRequestContext | null { return this.state.status === 'ready' ? this.state.data : null; }
  get latestRules(): OwnerRuleRevisionDto[] { return latestOwnerRules(this.context?.rules ?? []); }
  get dirty(): boolean { return this.context !== null && this.#snapshot() !== this.#baseline; }
  get outdatedRules(): RuleRefDto[] {
    return this.preservedRules.filter((ref) => this.latestRules.some((rule) => rule.rule_id === ref.rule_id && rule.revision !== ref.revision));
  }
  get unavailableRules(): RuleRefDto[] {
    return this.preservedRules.filter((ref) => !this.context?.rules.some((rule) => rule.rule_id === ref.rule_id && rule.revision === ref.revision));
  }
  get canReview(): boolean { return Boolean(this.intent.trim() && this.example.trim() && !this.outdatedRules.length && !this.unavailableRules.length && this.context?.app.archived_at === null); }

  cancel(): void {
    ++this.#generation;
    this.appId = null;
    this.changeId = null;
    this.draft = null;
    this.intent = this.example = this.scope = '';
    this.preservedRules = [];
    this.#baseline = '';
    this.state = loading();
    this.saving = this.saved = this.conflict = false;
    this.error = null;
  }

  async load(appId: string, changeId?: string): Promise<void> {
    this.cancel();
    const generation = this.#generation;
    this.appId = appId;
    this.changeId = changeId ?? this.#deps.uuid();
    try {
      const [context, rules] = await Promise.all([
        changeId ? this.#deps.getReview(appId, changeId) : this.#deps.getApp(appId).then((app) => ({ app, change: null })),
        ownerPages((page) => this.#deps.listRules(appId, page)),
      ]);
      if (!this.#matches(appId, generation)) return;
      this.draft = context.change;
      this.intent = context.change?.intent ?? '';
      this.example = context.change?.example ?? '';
      this.scope = context.change?.scope ?? '';
      this.preservedRules = context.change?.preserved_rules.map((ref) => ({ ...ref })) ?? latestOwnerRules(rules).map(({ rule_id, revision }) => ({ rule_id, revision }));
      this.state = ready({ app: context.app, rules });
      this.#baseline = this.#snapshot();
    } catch (reason) {
      if (this.#matches(appId, generation)) this.state = failed(toAppError(reason).message, () => void this.load(appId, changeId));
    }
  }

  toggleRule(rule: OwnerRuleRevisionDto, selected: boolean): void {
    this.preservedRules = this.preservedRules.filter((ref) => ref.rule_id !== rule.rule_id);
    if (selected) this.preservedRules.push({ rule_id: rule.rule_id, revision: rule.revision });
    this.saved = false;
  }

  useLatestRules(): void {
    this.preservedRules = this.preservedRules.map((ref) => {
      const latest = this.latestRules.find((rule) => rule.rule_id === ref.rule_id);
      return latest ? { rule_id: latest.rule_id, revision: latest.revision } : ref;
    });
    this.saved = false;
  }

  async save(state: ChangeState = 'draft'): Promise<boolean> {
    const appId = this.appId;
    const changeId = this.changeId;
    const generation = this.#generation;
    if (!appId || !changeId || !this.context || this.saving || this.conflict || this.context.app.archived_at !== null) return false;
    if (state === 'ready_for_review' && !this.canReview) return false;
    // Capture the submitted revision. Typing during the request remains unsaved.
    const submitted = this.#snapshot();
    const input = { operation_id: this.#deps.uuid(), id: changeId, app_id: appId, expected_version: this.draft?.version ?? 0,
      intent: this.intent, example: this.example, scope: this.scope, preserved_rules: this.preservedRules.map((ref) => ({ ...ref })),
      candidate_id: this.draft?.candidate_id ?? null, state };
    this.saving = true;
    this.error = null;
    this.saved = false;
    try {
      const draft = await this.#deps.save(input);
      if (!this.#matches(appId, generation) || this.changeId !== changeId) return false;
      this.draft = draft;
      this.#baseline = submitted;
      this.saved = true;
      return true;
    } catch (reason) {
      if (this.#matches(appId, generation)) {
        const error = toAppError(reason);
        this.error = error.message;
        this.conflict = /conflict/i.test(error.code) || /version|changed elsewhere|conflict/i.test(error.message);
      }
      return false;
    } finally {
      if (this.#matches(appId, generation)) this.saving = false;
    }
  }

  /** Keep all owner wording when resolving a conflict as a separate request. */
  saveAsNew(): void {
    if (!this.context || this.saving) return;
    this.changeId = this.#deps.uuid();
    this.draft = null;
    this.conflict = false;
    this.error = null;
    this.saved = false;
    this.#baseline = '';
  }

  #matches(appId: string, generation: number): boolean { return this.appId === appId && this.#generation === generation; }
  #snapshot(): string { return JSON.stringify([this.intent, this.example, this.scope, this.preservedRules]); }
}

export const changeRequest = new ChangeRequestStore();
